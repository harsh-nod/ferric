//! Full-size source authentication and bounded streaming uploads, without device buffers.

use super::super::super::packed_down_r1::{WeightSource, WeightSourceBytes};
use super::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const BYTES: usize = 4096 * 12_288 * 2;
const FIRST: u64 = 1_000_000;

struct Sources {
    bytes: Vec<u8>,
    records: Vec<WeightSource>,
    packed: Vec<[u8; 32]>,
    calls: usize,
    corrupt: bool,
}

impl Sources {
    fn new() -> Self {
        let bytes = vec![0; BYTES];
        let mut prefix = Sha256::new();
        prefix.update(&bytes[..BYTES - 16]);
        let mut source_prefix = prefix.clone();
        source_prefix.update([0; 8]);
        let mut records = Vec::new();
        let mut packed = Vec::new();
        for layer in 0..36 {
            let marker = (u64::from(layer) + 1).to_le_bytes();
            let mut digest = source_prefix.clone();
            digest.update(marker);
            let mut tail = [0; 16];
            for lane in 0..4 {
                tail[lane * 4 + 2..lane * 4 + 4].copy_from_slice(&marker[lane * 2..lane * 2 + 2]);
            }
            let mut expected = prefix.clone();
            expected.update(tail);
            packed.push(expected.finalize().into());
            records.push(WeightSource {
                layer,
                original: Tensor {
                    id: FIRST + u64::from(layer),
                    elements: BYTES / 2,
                    element_bytes: 2,
                },
                rows: (0, 4096),
                columns: (0, 12_288),
                range: (u64::from(layer) * BYTES as u64, BYTES as u64),
                sha256: digest.finalize().into(),
            });
        }
        Self {
            bytes,
            records,
            packed,
            calls: 0,
            corrupt: false,
        }
    }
}

impl WeightSourceBytes for Sources {
    fn with_source<T>(
        &mut self,
        source: &WeightSource,
        consume: impl FnOnce(&[u8]) -> TpResult<T>,
    ) -> TpResult<T> {
        self.calls += 1;
        self.bytes[BYTES - 8..].copy_from_slice(&(u64::from(source.layer) + 1).to_le_bytes());
        if self.corrupt {
            self.bytes[BYTES - 1] ^= 1;
        }
        consume(&self.bytes)
    }
}

#[derive(Default)]
struct Sink {
    allocations: Vec<usize>,
    uploads: Vec<(usize, Sha256)>,
    allocation_attempts: usize,
    alias: Option<u64>,
    fail_allocate: bool,
    fail_allocate_at: Option<usize>,
    fail_write: bool,
    fail_write_at: Option<(usize, usize)>,
}

impl EngineeringTpRankTransportV1 for Sink {
    fn allocate(&mut self, bytes: usize) -> TpResult<u64> {
        let ordinal = self.allocation_attempts;
        self.allocation_attempts += 1;
        if self.fail_allocate || self.fail_allocate_at == Some(ordinal) {
            return Err("injected packed down allocation failure".into());
        }
        self.allocations.push(bytes);
        self.uploads.push((0, Sha256::new()));
        Ok(self.alias.unwrap_or(self.allocations.len() as u64 + 10_000))
    }
    fn write(&mut self, id: u64, offset: usize, bytes: &[u8]) -> TpResult<()> {
        let index = self.allocations.len() - 1;
        assert_eq!(id, self.allocations.len() as u64 + 10_000);
        assert_eq!(offset, self.uploads[index].0);
        assert_eq!(bytes.len(), 1 << 20);
        assert!(offset + bytes.len() <= self.allocations[index]);
        if self.fail_write || self.fail_write_at == Some((index, offset)) {
            return Err("injected packed down upload failure".into());
        }
        self.uploads[index].0 += bytes.len();
        self.uploads[index].1.update(bytes);
        Ok(())
    }
    fn read(&mut self, _: u64, _: usize, _: &mut [u8]) -> TpResult<()> {
        panic!("setup must not read GPU data")
    }
    fn submit(&mut self, _: &EngineeringTpDispatchV1) -> TpResult<()> {
        panic!("setup must not dispatch")
    }
    fn wait(&mut self) -> TpResult<()> {
        panic!("setup must not wait")
    }
    fn close(&mut self) -> TpResult<()> {
        Ok(())
    }
}

fn prepare_sources(sink: &mut Sink, sources: &mut Sources) -> TpResult<Workspace> {
    let records = sources.records.clone();
    Workspace::prepare_sources(
        sink,
        &BTreeSet::from([999_999]),
        PackedDownBindingR1::recording(),
        &records,
        sources,
    )
}

#[test]
fn packed_down_setup_uploads_36_distinct_full_shape_authenticated_matrices() {
    let mut sources = Sources::new();
    let mut sink = Sink::default();
    let workspace = prepare_sources(&mut sink, &mut sources).unwrap();
    assert_eq!(sources.calls, 36);
    assert_eq!(workspace.bytes, 3_623_878_656);
    assert_eq!(workspace.selected, None);
    assert!(workspace.batch.is_none());
    assert_eq!(workspace.weights.len(), 36);
    assert_eq!(sink.allocations.len(), 37);
    assert_eq!(sink.allocations[..36], [BYTES; 36]);
    assert_eq!(sink.allocations[36], 24_576);
    assert_eq!(
        (
            workspace.scratch.id,
            workspace.scratch.elements,
            workspace.scratch.element_bytes
        ),
        (10_037, 6144, 4)
    );
    for layer in 0..36 {
        let weight = &workspace.weights[&u32::try_from(layer).unwrap()];
        assert_eq!(weight.source_sha256, sources.records[layer].sha256);
        assert_eq!(weight.original.id, FIRST + layer as u64);
        assert_eq!(
            (
                weight.packed.id,
                weight.packed.elements,
                weight.packed.element_bytes
            ),
            (10_001 + layer as u64, BYTES / 4, 4)
        );
        assert_eq!(sink.uploads[layer].0, BYTES);
        let digest: [u8; 32] = sink.uploads[layer].1.clone().finalize().into();
        assert_eq!(digest, sources.packed[layer]);
    }
    assert_eq!(sink.uploads[36].0, 0);
}

#[test]
fn packed_down_setup_rejects_roster_and_digest_before_allocation() {
    let mut sources = Sources::new();
    sources.records[1].original.id = sources.records[0].original.id;
    let mut sink = Sink::default();
    assert!(prepare_sources(&mut sink, &mut sources).is_err());
    assert_eq!(sources.calls, 0);
    assert!(sink.allocations.is_empty());
    sources.records[1].original.id = FIRST + 1;
    sources.corrupt = true;
    assert!(prepare_sources(&mut sink, &mut sources).is_err());
    assert_eq!(sources.calls, 1);
    assert!(sink.allocations.is_empty());
}

#[test]
fn packed_down_setup_stops_at_retained_alias_allocation_and_upload_failure() {
    let mut sources = Sources::new();
    for mutation in 0..4 {
        let mut sink = Sink {
            alias: match mutation {
                0 => Some(FIRST + 35),
                1 => Some(999_999),
                _ => None,
            },
            fail_allocate: mutation == 2,
            fail_write: mutation == 3,
            ..Sink::default()
        };
        assert!(prepare_sources(&mut sink, &mut sources).is_err());
        assert!(sink.allocations.len() <= 1);
        assert!(sink.uploads.iter().all(|upload| upload.0 == 0));
    }
}

#[test]
fn packed_down_setup_partial_later_upload_stops_without_publishing_workspace() {
    let mut sources = Sources::new();
    let mut sink = Sink {
        fail_write_at: Some((1, 2 << 20)),
        ..Sink::default()
    };
    let error = match prepare_sources(&mut sink, &mut sources) {
        Ok(_) => panic!("partial upload must not publish a workspace"),
        Err(error) => error,
    };
    assert!(error.contains("injected packed down upload failure"));
    assert_eq!(sources.calls, 2);
    assert_eq!(sink.allocation_attempts, 2);
    assert_eq!(sink.allocations, [BYTES, BYTES]);
    assert_eq!(sink.uploads[0].0, BYTES);
    let completed_digest: [u8; 32] = sink.uploads[0].1.clone().finalize().into();
    assert_eq!(completed_digest, sources.packed[0]);
    assert_eq!(sink.uploads[1].0, 2 << 20);
}

#[test]
fn packed_down_setup_scratch_allocation_failure_never_publishes_complete_weights() {
    let mut sources = Sources::new();
    let mut sink = Sink {
        fail_allocate_at: Some(36),
        ..Sink::default()
    };
    let error = match prepare_sources(&mut sink, &mut sources) {
        Ok(_) => panic!("missing activation scratch must not publish a workspace"),
        Err(error) => error,
    };
    assert!(error.contains("injected packed down allocation failure"));
    assert_eq!(sources.calls, 36);
    assert_eq!(sink.allocation_attempts, 37);
    assert_eq!(sink.allocations, [BYTES; 36]);
    assert_eq!(sink.uploads.len(), 36);
    for (layer, (written, digest)) in sink.uploads.iter().enumerate() {
        assert_eq!(*written, BYTES);
        let actual: [u8; 32] = digest.clone().finalize().into();
        assert_eq!(actual, sources.packed[layer]);
    }
}

#[test]
fn packed_down_setup_failure_closes_and_poison_driver() {
    let mut driver = fixture(1, &wide_pool());
    let error = driver
        .finish_packed_down_setup(Err("injected source failure".into()))
        .unwrap_err();
    assert!(error.contains("injected source failure"));
    assert!(driver.poisoned && driver.inner.closed && driver.packed_down_r1.is_none());
    assert!(
        driver.inner.transports[0]
            .events
            .borrow()
            .contains(&Event::Close(0))
    );
}

#[test]
fn packed_down_setup_retains_primary_error_and_failed_close_until_retry() {
    let mut driver = fixture(1, &wide_pool());
    driver.inner.transports[0].failure = Some(Failure::Close);
    let error = driver
        .finish_packed_down_setup(Err("injected source failure".into()))
        .unwrap_err();
    assert!(error.contains("injected source failure"));
    assert!(error.contains("packed down setup close"));
    assert!(error.contains("rank 0: injected close failure"));
    assert!(driver.poisoned && driver.inner.closed && driver.packed_down_r1.is_none());
    assert!(
        !driver.inner.transports[0]
            .events
            .borrow()
            .contains(&Event::Close(0))
    );
    driver.inner.transports[0].failure = None;
    driver.close().unwrap();
    assert!(
        driver.inner.transports[0]
            .events
            .borrow()
            .contains(&Event::Close(0))
    );
    assert!(driver.poisoned && driver.inner.closed);
}
