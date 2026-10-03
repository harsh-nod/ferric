//! Explicit real-model CPU probe. The transport only retains byte digests.
//! Synthetic transposed IDs serve the old inert source recorder, not execution.

use super::super::*;
use crate::tp_execution::batched::EngineeringTpBatchExecutionV2;
use crate::tp_execution::{
    EngineeringTpDispatchV1, EngineeringTpRankTransportV1, EngineeringTpReductionModeV3,
    allocate_tensor,
};
use crate::tp_model::EngineeringQwenModelV1;
use crate::tp_paged::{
    EngineeringTpPagedLimitsV1, EngineeringTpPagedPoolV1, EngineeringTpPoolScopeV1,
};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    io::Write,
    path::{Path, PathBuf},
    rc::Rc,
    time::Instant,
};

struct Allocation {
    rank: u32,
    bytes: usize,
    written: usize,
    digest: Sha256,
}

#[derive(Default)]
struct CpuCatalog {
    allocations: BTreeMap<u64, Allocation>,
    closed: [bool; 2],
    forbidden_calls: usize,
}

struct HashOnly {
    rank: u32,
    catalog: Rc<RefCell<CpuCatalog>>,
}

impl HashOnly {
    fn deny<T>(&self) -> TpResult<T> {
        self.catalog.borrow_mut().forbidden_calls += 1;
        Err("authentic packing CPU probe forbids registration, device access and execution".into())
    }
}

impl EngineeringTpRankTransportV1 for HashOnly {
    fn peer_group_rank(&self) -> Option<(u32, u32, u32)> {
        // A logical CPU fixture identifier, not a native child/process claim.
        Some((1, self.rank, 2))
    }
    fn supports_peer_dependency_collectives(&self) -> bool {
        true
    }
    fn setup_upload_chunk_bytes(&self) -> usize {
        MAX_CHUNK
    }
    fn allocate(&mut self, bytes: usize) -> TpResult<u64> {
        let mut catalog = self.catalog.borrow_mut();
        if catalog.closed[self.rank as usize] || bytes == 0 || catalog.allocations.len() >= 4096 {
            return Err("CPU packing allocation extent/lifecycle".into());
        }
        let id = catalog.allocations.len() as u64 + 1;
        catalog.allocations.insert(
            id,
            Allocation {
                rank: self.rank,
                bytes,
                written: 0,
                digest: Sha256::new(),
            },
        );
        Ok(id)
    }
    fn allocate_peer_readable(&mut self, bytes: usize) -> TpResult<u64> {
        self.allocate(bytes)
    }
    fn write(&mut self, id: u64, offset: usize, bytes: &[u8]) -> TpResult<()> {
        let mut catalog = self.catalog.borrow_mut();
        if catalog.closed[self.rank as usize] || bytes.is_empty() || bytes.len() > MAX_CHUNK {
            return Err("CPU packing write lifecycle/chunk".into());
        }
        let allocation = catalog
            .allocations
            .get_mut(&id)
            .ok_or("CPU packing unknown allocation")?;
        let end = offset
            .checked_add(bytes.len())
            .ok_or("CPU packing offset overflow")?;
        if allocation.rank != self.rank || offset != allocation.written || end > allocation.bytes {
            return Err("CPU packing write owner/order/extent".into());
        }
        allocation.digest.update(bytes);
        allocation.written = end;
        Ok(())
    }
    fn read(&mut self, _: u64, _: usize, _: &mut [u8]) -> TpResult<()> {
        self.deny()
    }
    fn submit(&mut self, _: &EngineeringTpDispatchV1) -> TpResult<()> {
        self.deny()
    }
    fn wait(&mut self) -> TpResult<()> {
        self.deny()
    }
    fn register_prepared_peer(
        &mut self,
        _: &crate::tp_execution::EngineeringTp2PreparedProgramV1,
    ) -> TpResult<[u8; 32]> {
        self.deny()
    }
    fn close(&mut self) -> TpResult<()> {
        self.catalog.borrow_mut().closed[self.rank as usize] = true;
        Ok(())
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn path_env(name: &str) -> PathBuf {
    let value =
        PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("explicit {name} required")));
    assert!(value.is_absolute(), "{name} must be absolute");
    value
}

fn recording_only_transposes(driver: &mut EngineeringTpBatchExecutionV2<HashOnly>) {
    let mut transposes = Vec::new();
    let mut total = 0u64;
    for index in 0..2 {
        let rank = &driver.inner.ranks[index];
        let originals = rank
            .layers
            .iter()
            .flat_map(|layer| layer.weights.iter())
            .chain(rank.globals.iter())
            .filter(|(kind, _)| {
                matches!(
                    kind,
                    Qwen3TensorKind::QueryProjection
                        | Qwen3TensorKind::KeyProjection
                        | Qwen3TensorKind::ValueProjection
                        | Qwen3TensorKind::OutputProjection
                        | Qwen3TensorKind::GateProjection
                        | Qwen3TensorKind::UpProjection
                        | Qwen3TensorKind::DownProjection
                        | Qwen3TensorKind::LanguageModelHead
                )
            })
            .map(|(_, tensor)| *tensor)
            .collect::<Vec<_>>();
        let mut map = BTreeMap::new();
        for original in originals {
            let transpose =
                allocate_tensor(&mut driver.inner.transports[index], original.elements, 2).unwrap();
            assert!(map.insert(original.id, transpose).is_none());
            total += original.elements as u64 * 2;
        }
        transposes.push(map);
    }
    driver.projection =
        crate::tp_execution::projection::ProjectionPolicy::synthetic_mfma_ranks_for_recording(
            transposes, total,
        );
    driver.projection_configured = true;
}

fn source_rows<'a>(
    model: &'a EngineeringQwenModelV1,
    source: &EngineeringTp2FiniteSourcePartV1,
) -> (&'a [u8], usize, usize, usize, usize) {
    let binding = model
        .layout()
        .lookup(
            Qwen3ModelRole::Target8B,
            source.metadata.kind,
            source.metadata.layer,
        )
        .unwrap();
    assert_eq!(binding.metadata(), source.metadata);
    assert_eq!(binding.sha256(), source.source_sha256);
    assert_eq!(
        source.source_dimensions,
        [source.metadata.dimension_0, source.metadata.dimension_1]
    );
    let bytes = section_bytes(model.target_weights(), binding.destination_range()).unwrap();
    let [first_row, rows] = source.rows.map(|v| v as usize);
    let [first_column, columns] = source.columns.map(|v| v as usize);
    let stride = source.metadata.dimension_1 as usize * 2;
    assert!(rows > 0 && columns > 0);
    assert!(first_row + rows <= source.metadata.dimension_0 as usize);
    assert!(first_column + columns <= source.metadata.dimension_1 as usize);
    (bytes, first_row, rows, first_column * 2, stride)
}

fn check_geometry(source: &EngineeringTp2FiniteSourcePartV1, rank: u32) {
    use Qwen3TensorKind as K;
    let m = source.metadata;
    let (rows, columns) = match m.kind {
        K::QueryProjection
        | K::KeyProjection
        | K::ValueProjection
        | K::GateProjection
        | K::UpProjection => (
            [rank * (m.dimension_0 / 2), m.dimension_0 / 2],
            [0, m.dimension_1],
        ),
        K::OutputProjection | K::DownProjection => (
            [0, m.dimension_0],
            [rank * (m.dimension_1 / 2), m.dimension_1 / 2],
        ),
        _ => ([0, m.dimension_0], [0, m.dimension_1]),
    };
    assert_eq!(source.rows, rows);
    assert_eq!(source.columns, columns);
}

fn check_upload(
    model: &EngineeringQwenModelV1,
    catalog: &CpuCatalog,
    upload: &EngineeringTp2FiniteUploadV1<'_>,
    keys: &mut BTreeSet<String>,
) -> Value {
    let key = upload.key();
    let key_text = format!("{key:?}");
    assert!(keys.insert(key_text.clone()), "duplicate recipe key");
    let sources = upload.sources().collect::<Vec<_>>();
    let (layer, kinds) = match key {
        EngineeringTp2FiniteUploadKeyV1::Source { layer, kind, .. } => (layer, vec![kind]),
        EngineeringTp2FiniteUploadKeyV1::PackedQkv { layer, .. } => (
            layer,
            vec![
                Qwen3TensorKind::QueryProjection,
                Qwen3TensorKind::KeyProjection,
                Qwen3TensorKind::ValueProjection,
            ],
        ),
        EngineeringTp2FiniteUploadKeyV1::PackedHeadNorm { layer, .. } => (
            layer,
            vec![Qwen3TensorKind::QueryNorm, Qwen3TensorKind::KeyNorm],
        ),
    };
    assert_eq!(
        sources.iter().map(|s| s.metadata.kind).collect::<Vec<_>>(),
        kinds
    );
    assert!(sources.iter().all(|s| s.metadata.layer == layer));
    let mut expected_digest = Sha256::new();
    let mut expected_bytes = 0usize;
    for source in &sources {
        check_geometry(source, key.rank());
        let (data, first, rows, column, stride) = source_rows(model, source);
        let row_bytes = source.columns[1] as usize * 2;
        for row in first..first + rows {
            expected_digest.update(&data[row * stride + column..row * stride + column + row_bytes]);
        }
        expected_bytes += rows * row_bytes;
    }
    assert_eq!(expected_bytes, upload.bytes());
    assert_eq!(
        <[u8; 32]>::from(expected_digest.finalize()),
        upload.sha256()
    );
    // Compare each emitted byte against independent scalar source row offsets,
    // not against another invocation of the production TP2 copy helper.
    let mut emitted = 0usize;
    let mut chunks = 0usize;
    let mut streamed_digest = Sha256::new();
    upload
        .visit_chunks(MAX_CHUNK, |offset, chunk| {
            assert_eq!(offset, emitted);
            let mut relative = offset;
            let mut consumed = 0;
            for source in &sources {
                let (data, first, rows, column, stride) = source_rows(model, source);
                let width = source.columns[1] as usize * 2;
                let extent = rows * width;
                if relative >= extent {
                    relative -= extent;
                    continue;
                }
                while relative < extent && consumed < chunk.len() {
                    let row = relative / width;
                    let inside = relative % width;
                    let count = (width - inside).min(chunk.len() - consumed);
                    let start = (first + row) * stride + column + inside;
                    assert_eq!(
                        &chunk[consumed..consumed + count],
                        &data[start..start + count]
                    );
                    consumed += count;
                    relative += count;
                }
                if consumed == chunk.len() {
                    break;
                }
                relative = 0;
            }
            assert_eq!(consumed, chunk.len());
            streamed_digest.update(chunk);
            emitted += chunk.len();
            chunks += 1;
            Ok(())
        })
        .unwrap();
    assert_eq!(emitted, upload.bytes());
    assert_eq!(
        <[u8; 32]>::from(streamed_digest.finalize()),
        upload.sha256()
    );
    let original_intake_join = if let EngineeringTp2FiniteUploadKeyV1::Source { rank, id, .. } = key
    {
        let allocation = catalog.allocations.get(&id).unwrap();
        assert_eq!(allocation.rank, rank);
        assert_eq!(allocation.bytes, upload.bytes());
        assert_eq!(allocation.written, upload.bytes());
        assert_eq!(
            <[u8; 32]>::from(allocation.digest.clone().finalize()),
            upload.sha256()
        );
        true
    } else {
        false
    };
    json!({"key":key_text,"rank":key.rank(),"bytes":emitted,"sha256":hex(&upload.sha256()),
        "chunks":chunks,"independent_source_bytes_equal":true,"original_intake_digest_equal":original_intake_join,
        "sources":sources.iter().map(|s| json!({"kind":format!("{:?}",s.metadata.kind),
            "layer":s.metadata.layer,"sha256":hex(&s.source_sha256),"dimensions":s.source_dimensions,
            "rows":s.rows,"columns":s.columns})).collect::<Vec<_>>()})
}

#[test]
#[ignore = "CPU-only authentic Qwen bundle, ~17GiB target memory; explicit source/report paths and bounded controller required"]
fn authentic_model_all_939_finite_uploads() {
    let start = Instant::now();
    let source = path_env("FERRIC_P221_SOURCE_BUNDLE");
    assert_eq!(std::fs::canonicalize(&source).unwrap(), source);
    let report_path = path_env("FERRIC_P221_PACKING_REPORT");
    assert!(!report_path.exists(), "report must be create-new");
    let parent = report_path.parent().unwrap();
    assert_eq!(std::fs::canonicalize(parent).unwrap(), parent);
    let model = EngineeringQwenModelV1::open(&source)
        .expect("existing canonical authenticated model intake");
    let intake_ms = start.elapsed().as_millis();
    println!("P221_AUTHENTIC_PACKING stage=model_authenticated elapsed_ms={intake_ms}");
    let pool = EngineeringTpPagedPoolV1::new(
        EngineeringTpPoolScopeV1 {
            model: *model.config().model_id.as_bytes(),
            session: [0x71; 32],
        },
        EngineeringTpPagedLimitsV1::new(2304, 32, 144, 100).unwrap(),
    )
    .unwrap();
    let catalog = Rc::new(RefCell::new(CpuCatalog::default()));
    let transports = (0..2)
        .map(|rank| HashOnly {
            rank,
            catalog: catalog.clone(),
        })
        .collect();
    let mut driver = EngineeringTpBatchExecutionV2::new(
        transports,
        model.config(),
        model.target_weights(),
        model.layout(),
        &pool,
    )
    .unwrap();
    let owner_ms = start.elapsed().as_millis();
    recording_only_transposes(&mut driver);
    driver
        .configure_reduction(EngineeringTpReductionModeV3::DevicePeerDependencyV1)
        .unwrap();
    let composition = driver.prepare_finite_two_forward_composition_v1().unwrap();
    assert_eq!(composition.layers().len(), 72);
    assert_eq!(composition.globals().len(), 3);
    assert!(
        EngineeringTp2FiniteWeightSourceV1::new(
            model.layout(),
            &model.target_weights()[..model.target_weights().len() - 2],
            &composition
        )
        .is_err()
    );
    let factory = EngineeringTp2FiniteWeightSourceV1::new(
        model.layout(),
        model.target_weights(),
        &composition,
    )
    .unwrap();
    let verified_ms = start.elapsed().as_millis();
    let mut rows = Vec::with_capacity(939);
    let mut keys = BTreeSet::new();
    for rank in 0..2 {
        for layer in 0..36 {
            let uploads = factory.layer_uploads(rank, layer).unwrap();
            assert_eq!(uploads.len(), 13);
            for upload in &uploads {
                assert_eq!(upload.key().rank(), rank);
                assert!(
                    upload
                        .sources()
                        .all(|source| source.metadata.layer == layer)
                );
                rows.push(check_upload(&model, &catalog.borrow(), upload, &mut keys));
            }
            assert!(
                start.elapsed().as_secs() < 3600,
                "CPU packing one-hour internal phase bound"
            );
            println!(
                "P221_AUTHENTIC_PACKING rank={rank} layer={layer} recipes={} elapsed_ms={}",
                rows.len(),
                start.elapsed().as_millis()
            );
        }
    }
    let globals = factory.global_uploads().unwrap();
    assert_eq!(globals.len(), 3);
    for upload in &globals {
        assert_eq!(upload.key().rank(), 0);
        assert!(
            upload
                .sources()
                .all(|source| source.metadata.layer == QWEN3_NO_LAYER)
        );
        rows.push(check_upload(&model, &catalog.borrow(), upload, &mut keys));
    }
    assert_eq!(rows.len(), 939);
    assert_eq!(keys.len(), 939);
    let total_bytes: u64 = rows.iter().map(|v| v["bytes"].as_u64().unwrap()).sum();
    assert_eq!(
        rows.iter()
            .filter(|v| v["original_intake_digest_equal"].as_bool() == Some(true))
            .count(),
        795
    );
    assert_eq!(catalog.borrow().forbidden_calls, 0);
    let bundle = hex(model.bundle_id().as_bytes());
    let model_id = hex(model.config().model_id.as_bytes());
    drop(globals);
    drop(factory);
    drop(composition);
    driver.close().unwrap();
    assert_eq!(catalog.borrow().closed, [true, true]);
    let report = json!({"schema":"ferric-p221-authentic-packing-cpu-v1","passed":true,
        "source_root":source,"bundle_id":bundle,"model_id":model_id,
        "target_bytes":model.target_weights().len(),"recipes":939,"original_intake_joins":795,
        "initialized_payload_bytes":total_bytes,"uploads":rows,
        "phase_elapsed_ms":{"model_authenticated":intake_ms,"cpu_intake_owner":owner_ms,
            "factory_sections_verified":verified_ms,"complete":start.elapsed().as_millis()},
        "gpu_execution":false,"native_upload":false,"source_grammar_transposes_are_metadata_only":true,
        "image_admission":false,"whole_model_execution":false,"inference_performance":false,
        "forbidden_transport_calls":0,"cpu_transports_closed":true});
    let data = serde_json::to_vec(&report).unwrap();
    assert!(data.len() < 4 * 1024 * 1024);
    write_exclusive(&report_path, &data);
    println!(
        "P221_AUTHENTIC_PACKING complete recipes=939 payload_bytes={total_bytes} report_sha256={}",
        hex(&Sha256::digest(&data))
    );
}

fn write_exclusive(path: &Path, data: &[u8]) {
    use std::os::unix::fs::OpenOptionsExt;
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .custom_flags(0x20000)
        .open(path)
        .unwrap(); // Linux O_NOFOLLOW.
    output.write_all(data).unwrap();
    output.sync_all().unwrap();
}
