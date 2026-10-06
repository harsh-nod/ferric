use super::super::{Backend, MLP_BYTES, PREFIX_BYTES, coordinate};
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Token {
    id: usize,
    rank: usize,
    bytes: u64,
}

impl Allocation for Token {
    fn owner(self) -> usize {
        self.rank
    }
    fn bytes(self) -> u64 {
        self.bytes
    }
}

fn roots() -> LayerBindings<Token> {
    let prefix: [[Token; 14]; 2] = std::array::from_fn(|rank| {
        std::array::from_fn(|index| Token {
            id: rank * 100 + index,
            rank,
            bytes: PREFIX_BYTES[index],
        })
    });
    let mlp = std::array::from_fn(|rank| {
        std::array::from_fn(|index| match index {
            5 => prefix[rank][7],
            9 => prefix[rank][13],
            _ => Token {
                id: rank * 100 + 20 + index,
                rank,
                bytes: MLP_BYTES[index],
            },
        })
    });
    LayerBindings {
        prefix,
        mlp,
        final_hidden: [prefix[0][0], prefix[1][0]],
    }
}

fn metadata(first_page: u32) -> Vec<u8> {
    std::iter::once(0u32)
        .chain((0..144).map(|i| (i + first_page) % 144))
        .flat_map(u32::to_le_bytes)
        .collect()
}

struct Reads {
    boundary: Boundary,
    calls: Vec<(usize, u64, u32)>,
    fail: Option<usize>,
    short: Option<usize>,
    bad_bits: Option<(usize, Boundary)>,
    metadata: Vec<u8>,
}

impl Default for Reads {
    fn default() -> Self {
        Self {
            boundary: Boundary::BeforePrefix,
            calls: vec![],
            fail: None,
            short: None,
            bad_bits: None,
            metadata: metadata(17),
        }
    }
}

impl Reader<Token> for Reads {
    fn read(&mut self, root: Token, offset: u64, bytes: u32) -> Result<Vec<u8>> {
        let index = self.calls.len();
        self.calls.push((root.id, offset, bytes));
        if self.fail == Some(index) {
            return Err("injected diagnostic read failure".into());
        }
        let mut data = if root.id % 100 == 5 {
            self.metadata.clone()
        } else if root.id % 100 == 13 {
            let value = if self.boundary == Boundary::AfterPrefix {
                1.5f32
            } else {
                -0.0f32
            };
            value.to_le_bytes().repeat(bytes as usize / 4)
        } else if root.id % 100 == 4 {
            1.0f32.to_le_bytes().repeat(bytes as usize / 4)
        } else {
            let value = match self.boundary {
                Boundary::BeforePrefix => 0x3f80u16,
                Boundary::AfterPrefix => 0x4000,
                Boundary::AfterFirstResidual => 0x4040,
                Boundary::AfterMlp => 0x4080,
                Boundary::AfterFinalResidual => 0x40a0,
            };
            value.to_le_bytes().repeat(bytes as usize / 2)
        };
        if self.bad_bits == Some((root.id, self.boundary)) {
            if root.id % 100 == 13 {
                data[..4].copy_from_slice(&0x7fc0_0001u32.to_le_bytes());
            } else {
                data[..2].copy_from_slice(&0x7fc1u16.to_le_bytes());
            }
        }
        if self.short == Some(index) {
            data.pop();
        }
        Ok(data)
    }
}

fn complete(reads: &mut Reads) -> Result<Layer0CaptureV1> {
    let mut capture = Collector::new(1, 0, 0)?;
    let roots = roots();
    for boundary in BOUNDARIES {
        reads.boundary = boundary;
        capture.observe(boundary, reads, &roots)?;
    }
    capture.finish()
}

fn guarded_down() -> [Token; 2] {
    std::array::from_fn(|rank| Token { id: 200 + rank * 100 + 13, rank, bytes: 16384 })
}

#[test]
fn guarded_capture_reads_actual_down_without_replacing_prefix_partial() {
    let mut capture = Collector::new(1, 0, 0).unwrap();
    let mut reader = Reads::default();
    let roots = roots();
    let down = guarded_down();
    for boundary in BOUNDARIES {
        reader.boundary = boundary;
        let before = reader.calls.len();
        capture.observe_guarded(boundary, &mut reader, &roots, down).unwrap();
        for rank in 0..2 {
            let calls = &reader.calls[before..];
            if boundary == Boundary::AfterPrefix {
                assert!(calls.contains(&(roots.prefix[rank][13].id, 0, 16384)));
                assert!(!calls.iter().any(|v| v.0 == down[rank].id));
            } else if boundary == Boundary::AfterMlp {
                assert!(calls.contains(&(down[rank].id, 0, 16384)));
                assert!(!calls.iter().any(|v| v.0 == roots.mlp[rank][9].id));
            }
        }
    }
    let result = capture.finish().unwrap();
    assert_eq!(reader.calls.len(), PARTS);
    assert_eq!(result.payload.len(), PAYLOAD_BYTES);
}

#[test]
fn guarded_capture_stale_down_and_read_failure_prevent_publication() {
    let roots = roots();
    for rank in 0..2 {
        let mut collector = Collector::new(1, 0, 0).unwrap();
        let mut reader = Reads::default();
        let mut down = guarded_down();
        down[rank] = roots.mlp[rank][9];
        assert!(collector.observe_guarded(Boundary::BeforePrefix, &mut reader, &roots, down).is_err());
        assert!(reader.calls.is_empty());
        assert!(collector.observe_guarded(Boundary::BeforePrefix, &mut reader, &roots, guarded_down()).is_err());
        assert!(collector.finish().is_err());
    }
    for fail in 0..PARTS {
        let mut collector = Collector::new(1, 0, 0).unwrap();
        let mut reader = Reads { fail: Some(fail), ..Reads::default() };
        for boundary in BOUNDARIES {
            reader.boundary = boundary;
            if collector.observe_guarded(boundary, &mut reader, &roots, guarded_down()).is_err() { break; }
        }
        assert_eq!(reader.calls.len(), fail + 1);
        assert!(collector.finish().is_err());
    }
}

#[test]
fn guarded_capture_close_confirmation_requires_successful_close() {
    let mut called = 0;
    let result = complete(&mut Reads::default()).unwrap().after_close(|| {
        called += 1;
        Err("injected Close failure".into())
    });
    assert!(result.is_err());
    assert_eq!(called, 1);
    let closed = complete(&mut Reads::default()).unwrap().after_close(|| { called += 1; Ok(()) }).unwrap();
    assert_eq!(called, 2);
    assert!(closed.native_close_confirmed);
    assert!(!closed.numerical_acceptance && !closed.performance_claim && !closed.production_authority);
}

fn part<'a>(capture: &'a Layer0CaptureV1, role: Role, rank: u32) -> (&'a Part, &'a [u8]) {
    let part = capture
        .parts
        .iter()
        .find(|part| part.role == role && part.rank == rank)
        .unwrap();
    let start = part.offset as usize;
    (part, &capture.payload[start..start + part.bytes as usize])
}

#[test]
fn exact_34_part_256136_byte_abi_hashes_and_scope_are_fixed() {
    let mut reader = Reads::default();
    let capture = complete(&mut reader).unwrap();
    assert_eq!(reader.calls.len(), 34);
    assert_eq!(capture.parts.len(), 34);
    assert_eq!(capture.payload.len(), 256_136);
    assert_eq!(capture.payload_bytes, 256_136);
    let digest: [u8; 32] = Sha256::digest(&capture.payload).into();
    assert_eq!(capture.payload_sha256, digest);
    let mut offset = 0;
    for part in &capture.parts {
        assert_eq!(part.offset, offset);
        assert_eq!(part.bytes, part.elements * part.scalar.bytes());
        let data = &capture.payload[offset as usize..(offset + part.bytes) as usize];
        let digest: [u8; 32] = Sha256::digest(data).into();
        assert_eq!(part.sha256, digest);
        offset += part.bytes;
    }
    assert_eq!(offset, 256_136);
    assert_eq!(
        (capture.generation, capture.position, capture.layer),
        (1, 0, 0)
    );
    let json = serde_json::to_value(&capture).unwrap();
    assert_eq!(json["schema"], "FerricFiniteLayerZeroCaptureV1");
    for key in [
        "full_cache_capture",
        "native_close_confirmed",
        "numerical_acceptance",
        "performance_claim",
        "production_authority",
    ] {
        assert_eq!(json[key], false);
    }
    assert_eq!(part(&capture, Role::CacheMetadata, 0).0.scalar, Scalar::U32);
    assert_eq!(part(&capture, Role::Rotary, 1).0.scalar, Scalar::F32);
    assert_eq!(part(&capture, Role::OutputPartial, 0).0.scalar, Scalar::F32);
    assert_eq!(part(&capture, Role::DownPartial, 1).0.scalar, Scalar::F32);
    assert!(serde_json::to_vec(&capture).unwrap().len() < 1_100_000);
}

#[test]
fn snapshots_preserve_pre_overwrite_normalized_partial_and_original_hidden_bits() {
    let capture = complete(&mut Reads::default()).unwrap();
    for rank in 0..2 {
        assert_eq!(
            &part(&capture, Role::Input, rank).1[..2],
            &0x3f80u16.to_le_bytes()
        );
        assert_eq!(
            &part(&capture, Role::FinalHidden, rank).1[..2],
            &0x40a0u16.to_le_bytes()
        );
        assert_eq!(
            &part(&capture, Role::InputNormalized, rank).1[..2],
            &0x4000u16.to_le_bytes()
        );
        assert_eq!(
            &part(&capture, Role::PostNormalized, rank).1[..2],
            &0x4080u16.to_le_bytes()
        );
        assert_eq!(
            &part(&capture, Role::OutputPartial, rank).1[..4],
            &1.5f32.to_le_bytes()
        );
        assert_eq!(
            &part(&capture, Role::DownPartial, rank).1[..4],
            &(-0.0f32).to_le_bytes()
        );
    }
}

#[test]
fn capture_scope_rejects_other_positions_layers_generations_and_missing_boundaries() {
    for scope in [
        (0, 0, 0),
        (2, 0, 0),
        (1, 1, 0),
        (1, 0, 1),
        (u64::MAX, u32::MAX, usize::MAX),
    ] {
        assert!(Collector::new(scope.0, scope.1, scope.2).is_err());
    }
    for count in 0..5 {
        let mut capture = Collector::new(1, 0, 0).unwrap();
        let mut reader = Reads::default();
        for boundary in BOUNDARIES.iter().copied().take(count) {
            reader.boundary = boundary;
            capture.observe(boundary, &mut reader, &roots()).unwrap();
        }
        assert!(capture.finish().is_err());
    }
}

#[test]
fn duplicate_or_out_of_order_boundary_is_terminal_before_another_read() {
    for boundary in BOUNDARIES {
        let mut capture = Collector::new(1, 0, 0).unwrap();
        let mut reader = Reads::default();
        if boundary == Boundary::BeforePrefix {
            capture.observe(boundary, &mut reader, &roots()).unwrap();
        }
        let before = reader.calls.len();
        assert!(capture.observe(boundary, &mut reader, &roots()).is_err());
        assert_eq!(reader.calls.len(), before);
        assert!(
            capture
                .observe(Boundary::AfterPrefix, &mut reader, &roots())
                .is_err()
        );
        assert_eq!(reader.calls.len(), before);
        assert!(capture.finish().is_err());
    }
}

#[test]
fn every_failed_or_short_read_prevents_complete_capture() {
    for index in 0..34 {
        for short in [false, true] {
            let mut reader = Reads::default();
            if short {
                reader.short = Some(index);
            } else {
                reader.fail = Some(index);
            }
            assert!(complete(&mut reader).is_err());
            assert_eq!(reader.calls.len(), index + 1);
        }
    }
}

#[test]
fn current_kv_offset_uses_full_validated_nonidentity_page_map_not_logical_zero() {
    for page in [0, 17, 143] {
        let mut reader = Reads {
            metadata: metadata(page),
            ..Reads::default()
        };
        let capture = complete(&mut reader).unwrap();
        for rank in 0..2 {
            for role in [Role::CurrentKey, Role::CurrentValue] {
                let (p, _) = part(&capture, role, rank);
                assert_eq!(p.source_byte_offset, u64::from(page) * 16 * 512 * 2);
                assert_eq!(p.bytes, 1024);
            }
        }
        assert_eq!(
            reader
                .calls
                .iter()
                .filter(
                    |(_, offset, bytes)| *offset == u64::from(page) * 16 * 1024 && *bytes == 1024
                )
                .count(),
            4
        );
    }
    for mutation in 0..3 {
        let mut reader = Reads::default();
        match mutation {
            0 => reader.metadata[..4].copy_from_slice(&1u32.to_le_bytes()),
            1 => reader.metadata[4..8].copy_from_slice(&144u32.to_le_bytes()),
            _ => {
                let first = reader.metadata[4..8].to_vec();
                reader.metadata[576..580].copy_from_slice(&first);
            }
        }
        assert!(complete(&mut reader).is_err());
        assert_eq!(reader.calls.len(), 2);
    }
}

#[test]
fn nonfinite_bf16_and_f32_diagnostics_fail_without_lossy_bit_conversion() {
    for target in [
        (8, Boundary::AfterPrefix),
        (13, Boundary::AfterPrefix),
        (113, Boundary::AfterMlp),
    ] {
        let mut reader = Reads {
            bad_bits: Some(target),
            ..Reads::default()
        };
        assert!(complete(&mut reader).is_err());
    }
    for bits in [0u32, 0x8000_0000, 1, 0x7f7f_ffff] {
        finite(Scalar::F32, &bits.to_le_bytes()).unwrap();
    }
    for bits in [0u16, 0x8000, 1, 0x7f7f] {
        finite(Scalar::Bf16, &bits.to_le_bytes()).unwrap();
    }
}

#[test]
fn bad_root_owner_extent_and_alias_fail_before_the_first_read() {
    for mutation in 0..3 {
        let mut roots = roots();
        match mutation {
            0 => roots.prefix[0][8].rank = 1,
            1 => roots.prefix[0][8].bytes -= 1,
            _ => roots.prefix[0][8] = roots.prefix[0][7],
        }
        let mut reader = Reads::default();
        let mut capture = Collector::new(1, 0, 0).unwrap();
        assert!(
            capture
                .observe(Boundary::BeforePrefix, &mut reader, &roots)
                .is_err()
        );
        assert!(reader.calls.is_empty());
        assert!(capture.finish().is_err());
    }
}

struct Phases {
    events: Vec<&'static str>,
    fail: Option<Boundary>,
    poisoned: bool,
}

impl Backend for Phases {
    fn validate(&mut self) -> Result<()> {
        self.events.push("validate");
        Ok(())
    }
    fn capture(&mut self, boundary: Boundary) -> Result<()> {
        self.events.push(match boundary {
            Boundary::BeforePrefix => "capture-input",
            Boundary::AfterPrefix => "capture-prefix",
            Boundary::AfterFirstResidual => "capture-first",
            Boundary::AfterMlp => "capture-mlp",
            Boundary::AfterFinalResidual => "capture-final",
        });
        if self.fail == Some(boundary) {
            Err("capture read failed".into())
        } else {
            Ok(())
        }
    }
    fn prefix(&mut self) -> Result<([[u32; 22]; 2], [u64; 2])> {
        self.events.push("prefix-acquired");
        Ok(([[0; 22]; 2], [1, 2]))
    }
    fn first_residual(&mut self) -> Result<[u64; 2]> {
        self.events.push("first-complete");
        Ok([3, 4])
    }
    fn mlp(&mut self) -> Result<([[u32; 11]; 2], [u64; 2])> {
        self.events.push("mlp-acquired");
        Ok(([[0; 11]; 2], [5, 6]))
    }
    fn final_residual(&mut self) -> Result<[u64; 2]> {
        self.events.push("final-complete");
        Ok([7, 8])
    }
    fn poison(&mut self) {
        self.poisoned = true;
    }
}

#[test]
fn capture_occurs_after_completed_dependencies_and_before_scratch_reuse() {
    let mut fake = Phases {
        events: vec![],
        fail: None,
        poisoned: false,
    };
    let result = coordinate(&mut fake).unwrap();
    assert_eq!(
        fake.events,
        [
            "validate",
            "capture-input",
            "prefix-acquired",
            "capture-prefix",
            "first-complete",
            "capture-first",
            "mlp-acquired",
            "capture-mlp",
            "final-complete",
            "capture-final"
        ]
    );
    assert_eq!(result.paired_ns, [[1, 2], [3, 4], [5, 6], [7, 8]]);
    assert!(!fake.poisoned);
}

#[test]
fn every_capture_failure_stops_later_dispatches_and_poisons_the_owner() {
    for (index, boundary) in BOUNDARIES.into_iter().enumerate() {
        let mut fake = Phases {
            events: vec![],
            fail: Some(boundary),
            poisoned: false,
        };
        assert!(coordinate(&mut fake).is_err());
        assert_eq!(fake.events.len(), 2 + index * 2);
        assert!(fake.poisoned);
    }
}
