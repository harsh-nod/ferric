//! Full-shape preparation contracts, not model authentication or GPU arithmetic.

use super::super::super::packed_gate_up_r2::{WeightSource, WeightSourceBytes, admit_or_close};
use super::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const MATRIX_BYTES: usize = 12_288 * 4096 * 2;
const CHUNK: usize = 1 << 20;
const NORMALIZED: u64 = 999_999;
const FIRST_ORIGINAL: u64 = 1_000_000;
const FIRST_PACKED: u64 = 10_000;

struct Sources {
    bytes: Vec<u8>,
    records: Vec<WeightSource>,
    packed_digests: Vec<[u8; 32]>,
    calls: usize,
    corrupt: Option<usize>,
    short: bool,
}

impl Sources {
    fn new() -> Self {
        let bytes = vec![0; MATRIX_BYTES];
        let mut prefix = Sha256::new();
        prefix.update(&bytes[..MATRIX_BYTES - 16]);
        let mut source_prefix = prefix.clone();
        source_prefix.update([0; 8]);
        let mut records = Vec::new();
        let mut packed_digests = Vec::new();
        for ordinal in 0..72_u32 {
            let marker = u64::from(ordinal + 1).to_le_bytes();
            let mut source = source_prefix.clone();
            source.update(marker);
            let mut tail = [0; 16];
            for lane in 0..4 {
                tail[lane * 4 + 2..lane * 4 + 4].copy_from_slice(&marker[lane * 2..lane * 2 + 2]);
            }
            let mut packed = prefix.clone();
            packed.update(tail);
            packed_digests.push(packed.finalize().into());
            records.push(WeightSource {
                layer: ordinal / 2,
                kind: if ordinal % 2 == 0 {
                    Qwen3TensorKind::GateProjection
                } else {
                    Qwen3TensorKind::UpProjection
                },
                original: Tensor {
                    id: FIRST_ORIGINAL + u64::from(ordinal),
                    elements: 12_288 * 4096,
                    element_bytes: 2,
                },
                rows: (0, 12_288),
                columns: (0, 4096),
                range: (
                    u64::from(ordinal) * MATRIX_BYTES as u64,
                    MATRIX_BYTES as u64,
                ),
                sha256: source.finalize().into(),
            });
        }
        Self {
            bytes,
            records,
            packed_digests,
            calls: 0,
            corrupt: None,
            short: false,
        }
    }
}

impl WeightSourceBytes for Sources {
    fn with_source<T>(
        &mut self,
        source: &WeightSource,
        consume: impl FnOnce(&[u8]) -> TpResult<T>,
    ) -> TpResult<T> {
        let ordinal = usize::try_from(source.original.id - FIRST_ORIGINAL).unwrap();
        self.calls += 1;
        self.bytes[MATRIX_BYTES - 8..]
            .copy_from_slice(&u64::try_from(ordinal + 1).unwrap().to_le_bytes());
        if self.corrupt == Some(ordinal) {
            self.bytes[MATRIX_BYTES - 1] ^= 1;
        }
        let end = if self.short {
            MATRIX_BYTES - 2
        } else {
            MATRIX_BYTES
        };
        consume(&self.bytes[..end])
    }
}

struct Upload {
    id: u64,
    bytes: usize,
    written: usize,
    chunks: usize,
    digest: Sha256,
}

#[derive(Default)]
struct Sink {
    uploads: Vec<Upload>,
    allocation_calls: usize,
    write_calls: usize,
    fail_allocate: Option<usize>,
    fail_write: Option<(usize, usize)>,
    alias: Option<(usize, u64)>,
    closes: usize,
    close_failure: bool,
    loaded: bool,
    peer: bool,
}

impl EngineeringTpRankTransportV1 for Sink {
    fn require_loaded_image(&mut self, image: [u8; 32], roots: &[&str]) -> TpResult<()> {
        assert_eq!(image, PackedBf16BindingR2::recording().hsaco());
        assert_eq!(roots, ROOTS);
        if self.loaded {
            Ok(())
        } else {
            Err("injected unloaded image".into())
        }
    }

    fn peer_group_rank(&self) -> Option<(u32, u32, u32)> {
        self.peer.then_some((0, 1, 1))
    }

    fn allocate(&mut self, bytes: usize) -> TpResult<u64> {
        let index = self.allocation_calls;
        self.allocation_calls += 1;
        assert_eq!(bytes, if index == 72 { 8192 } else { MATRIX_BYTES });
        if self.fail_allocate == Some(index) {
            return Err("injected setup allocation failure".into());
        }
        let id = self
            .alias
            .filter(|(at, _)| *at == index)
            .map_or(FIRST_PACKED + index as u64, |(_, id)| id);
        self.uploads.push(Upload {
            id,
            bytes,
            written: 0,
            chunks: 0,
            digest: Sha256::new(),
        });
        Ok(id)
    }

    fn write(&mut self, id: u64, offset: usize, bytes: &[u8]) -> TpResult<()> {
        let index = self.uploads.len() - 1;
        let upload = self.uploads.last_mut().unwrap();
        assert_eq!(id, upload.id);
        assert_eq!(offset, upload.written);
        assert_eq!(bytes.len(), CHUNK);
        assert!(offset + bytes.len() <= upload.bytes);
        self.write_calls += 1;
        if self.fail_write == Some((index, upload.chunks)) {
            return Err("injected setup upload failure".into());
        }
        upload.digest.update(bytes);
        upload.written += bytes.len();
        upload.chunks += 1;
        Ok(())
    }

    fn read(&mut self, _: u64, _: usize, _: &mut [u8]) -> TpResult<()> {
        panic!("preparation must not read device buffers")
    }
    fn submit(&mut self, _: &EngineeringTpDispatchV1) -> TpResult<()> {
        panic!("preparation must not dispatch")
    }
    fn wait(&mut self) -> TpResult<()> {
        panic!("preparation must not wait for a dispatch")
    }
    fn close(&mut self) -> TpResult<()> {
        self.closes += 1;
        if self.close_failure {
            Err("injected setup close failure".into())
        } else {
            Ok(())
        }
    }
}

fn prepare_sources(
    transport: &mut impl EngineeringTpRankTransportV1,
    sources: &mut Sources,
) -> TpResult<Workspace> {
    let records = sources.records.clone();
    Workspace::prepare_sources(
        transport,
        NORMALIZED,
        PackedBf16BindingR2::recording(),
        &records,
        sources,
    )
}

#[test]
fn packed_gate_up_setup_uploads_72_distinct_full_shape_authenticated_matrices() {
    let mut sources = Sources::new();
    assert_eq!(
        sources
            .records
            .iter()
            .map(|record| record.sha256)
            .collect::<BTreeSet<_>>()
            .len(),
        72
    );
    let mut transport = Sink::default();
    let workspace = prepare_sources(&mut transport, &mut sources).unwrap();
    assert_eq!(sources.calls, 72);
    assert_eq!(workspace.weights.len(), 72);
    assert_eq!(workspace.bytes, 7_247_757_312);
    assert_eq!(workspace.selected, None);
    assert_eq!(workspace.binding, PackedBf16BindingR2::recording());
    assert_eq!(
        (
            workspace.scratch.id,
            workspace.scratch.elements,
            workspace.scratch.element_bytes
        ),
        (FIRST_PACKED + 72, 2048, 4)
    );
    assert_eq!(transport.allocation_calls, 73);
    assert_eq!(transport.write_calls, 72 * 96);
    for (ordinal, record) in sources.records.iter().enumerate() {
        let tag = if ordinal % 2 == 0 { 4 } else { 5 };
        let weight = &workspace.weights[&(record.layer, tag)];
        assert_eq!(
            (
                weight.original.id,
                weight.original.elements,
                weight.original.element_bytes
            ),
            (record.original.id, 12_288 * 4096, 2)
        );
        assert_eq!(
            (
                weight.packed.id,
                weight.packed.elements,
                weight.packed.element_bytes
            ),
            (FIRST_PACKED + ordinal as u64, 12_288 * 2048, 4)
        );
        assert_eq!(weight.source_sha256, record.sha256);
        let upload = &transport.uploads[ordinal];
        assert_eq!((upload.written, upload.chunks), (MATRIX_BYTES, 96));
        let digest: [u8; 32] = upload.digest.clone().finalize().into();
        assert_eq!(digest, sources.packed_digests[ordinal]);
    }
    assert_eq!(transport.uploads[72].written, 0);
}

#[test]
fn packed_gate_up_setup_rejects_incomplete_duplicate_or_invalid_rosters_before_allocating() {
    let mut sources = Sources::new();
    for mutation in 0..14 {
        let mut records = sources.records.clone();
        match mutation {
            0 => {
                records.pop();
            }
            1 => records.push(records[0]),
            2 => records[1].kind = Qwen3TensorKind::GateProjection,
            3 => records[0].layer = 36,
            4 => records[0].kind = Qwen3TensorKind::QueryProjection,
            5 => records[1].original.id = records[0].original.id,
            6 => records[0].rows = (1, 12_288),
            7 => records[0].columns = (0, 4095),
            8 => records[0].original.elements -= 1,
            9 => records[0].original.element_bytes = 4,
            10 => records[1].range.0 -= 2,
            11 => records[0].range.1 -= 2,
            12 => records[0].range.0 = u64::MAX,
            _ => records[1].range = records[0].range,
        }
        let mut transport = Sink::default();
        assert!(
            Workspace::prepare_sources(
                &mut transport,
                NORMALIZED,
                PackedBf16BindingR2::recording(),
                &records,
                &mut sources
            )
            .is_err()
        );
        assert_eq!(
            (
                transport.allocation_calls,
                transport.write_calls,
                sources.calls
            ),
            (0, 0, 0)
        );
    }
}

#[test]
fn packed_gate_up_setup_authenticates_actual_full_matrix_before_allocation() {
    let mut sources = Sources::new();
    for (short, ordinal) in [(false, 0), (false, 1), (true, 0)] {
        sources.corrupt = (!short).then_some(ordinal);
        sources.short = short;
        let mut transport = Sink::default();
        let error = prepare_sources(&mut transport, &mut sources).err().unwrap();
        assert!(error.contains("source digest or row extent"));
        assert_eq!(
            (transport.allocation_calls, transport.write_calls),
            (ordinal, ordinal * 96)
        );
    }
}

#[test]
fn packed_gate_up_setup_allocator_cannot_alias_future_sources_normalized_or_packed_storage() {
    let mut sources = Sources::new();
    for (index, id) in [
        (0, FIRST_ORIGINAL),
        (0, FIRST_ORIGINAL + 71),
        (0, NORMALIZED),
        (1, FIRST_PACKED),
    ] {
        sources.calls = 0;
        let mut transport = Sink {
            alias: Some((index, id)),
            ..Sink::default()
        };
        let error = prepare_sources(&mut transport, &mut sources).err().unwrap();
        assert!(error.contains("allocator repeated a live identity"));
        assert_eq!(transport.allocation_calls, index + 1);
        assert_eq!(transport.write_calls, index * 96);
        assert_eq!(sources.calls, index + 1);
    }
}

#[test]
fn packed_gate_up_setup_stops_at_allocation_and_partial_chunk_upload_failures() {
    let mut sources = Sources::new();
    for (allocation, write, calls, successful_chunks) in [
        (Some(0), None, 1, 0),
        (Some(1), None, 2, 96),
        (None, Some((0, 0)), 1, 0),
        (None, Some((0, 95)), 1, 95),
        (None, Some((1, 0)), 2, 96),
        (None, Some((1, 95)), 2, 191),
    ] {
        sources.calls = 0;
        let mut transport = Sink {
            fail_allocate: allocation,
            fail_write: write,
            ..Sink::default()
        };
        let error = prepare_sources(&mut transport, &mut sources).err().unwrap();
        assert!(error.contains("injected setup"));
        assert_eq!(sources.calls, calls);
        assert_eq!(
            transport
                .uploads
                .iter()
                .map(|upload| upload.chunks)
                .sum::<usize>(),
            successful_chunks
        );
        assert_eq!(transport.allocation_calls, calls);
        assert_eq!(transport.closes, 0);
    }
}

#[test]
fn packed_gate_up_setup_scratch_failure_never_returns_a_partial_workspace() {
    let mut sources = Sources::new();
    let mut transport = Sink {
        fail_allocate: Some(72),
        ..Sink::default()
    };
    let error = prepare_sources(&mut transport, &mut sources).err().unwrap();
    assert!(error.contains("injected setup allocation failure"));
    assert_eq!(sources.calls, 72);
    assert_eq!(transport.allocation_calls, 73);
    assert_eq!(transport.write_calls, 72 * 96);
}

#[test]
fn packed_gate_up_setup_failure_closes_actual_driver_and_retains_close_errors() {
    let mut sources = Sources::new();
    for (write_failure, close_failure) in
        [(false, false), (true, false), (false, true), (true, true)]
    {
        let mut driver = fixture(1, &wide_pool());
        let normalized = driver.inner.ranks[0].normalized.id;
        let transport = &mut driver.inner.transports[0];
        let before = transport.writes.len();
        transport.failure = Some(if write_failure {
            Failure::Write
        } else {
            Failure::AllocateAt(transport.next + 1)
        });
        let records = sources.records.clone();
        let prepared = Workspace::prepare_sources(
            transport,
            normalized,
            PackedBf16BindingR2::recording(),
            &records,
            &mut sources,
        );
        assert!(prepared.is_err());
        assert_eq!(
            transport.writes.len() - before,
            if write_failure { 0 } else { 96 }
        );
        transport.failure = close_failure.then_some(Failure::Close);
        let error = driver.finish_packed_gate_up_setup(prepared).unwrap_err();
        assert!(error.contains(if write_failure {
            "injected metadata write failure"
        } else {
            "injected allocation failure"
        }));
        assert_eq!(error.contains("packed gate/up setup close"), close_failure);
        assert!(driver.poisoned && driver.inner.closed && driver.packed_gate_up_r2.is_none());
        assert_eq!(driver.completed_batches, 0);
        assert!(driver.inner.transports[0].commands.is_empty());
        assert_eq!(
            driver.inner.transports[0]
                .events
                .borrow()
                .iter()
                .filter(|event| matches!(event, Event::Close(0)))
                .count(),
            usize::from(!close_failure)
        );
    }
}

#[test]
fn packed_gate_up_setup_image_admission_closes_every_rank_and_reports_all_close_failures() {
    for count in [0, 1, 2] {
        let mut transports = (0..count)
            .map(|_| Sink {
                close_failure: true,
                ..Sink::default()
            })
            .collect::<Vec<_>>();
        let error = admit_or_close(&mut transports, PackedBf16BindingR2::recording()).unwrap_err();
        assert!(
            transports
                .iter()
                .all(|transport| transport.closes == 1 && transport.allocation_calls == 0)
        );
        for index in 0..count {
            assert!(error.contains(&format!("rank {index}: injected setup close failure")));
        }
    }
    let mut transports = [Sink {
        loaded: true,
        ..Sink::default()
    }];
    admit_or_close(&mut transports, PackedBf16BindingR2::recording()).unwrap();
    assert_eq!(transports[0].closes, 0);
    transports[0].peer = true;
    assert!(admit_or_close(&mut transports, PackedBf16BindingR2::recording()).is_err());
    assert_eq!(transports[0].closes, 1);
}
