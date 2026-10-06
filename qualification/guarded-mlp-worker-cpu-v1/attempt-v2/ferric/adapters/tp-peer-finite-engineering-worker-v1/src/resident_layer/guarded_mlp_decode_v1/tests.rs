use super::*;
use fe2o3_kfd::engineering_wire::{BufferAccessV1, ExplicitArgumentV1};
fn meta(validator: bool) -> KernelMetadataV1 {
    let n = if validator { 1 } else { 6 };
    let explicit = if validator { 24 } else { 104 };
    KernelMetadataV1 {
        symbol: if validator { GUARD } else { R2 }.into(),
        object_sha256: GUARDED_IMAGE,
        kernarg_bytes: explicit + 256,
        kernarg_alignment: 8,
        wavefront_size: 64,
        private_segment_bytes: 0,
        group_segment_bytes: 0,
        implicit_argument_offset: Some(explicit),
        implicit_argument_bytes: 256,
        explicit_arguments: (0..n * 2 + 2)
            .map(|i| ExplicitArgumentV1 {
                offset: if i < n * 2 {
                    (i * 8) as u32
                } else {
                    (n * 16 + (i - n * 2) * 4) as u32
                },
                bytes: if i < n * 2 { 8 } else { 4 },
                global_buffer: i < n * 2 && i % 2 == 0,
                pointee_alignment: None,
                access: None,
            })
            .collect(),
    }
}
#[test]
fn guarded_model_images_reject_wrong_bytes_and_every_physical_argument_drift() {
    let pin = crate::finite_setup_wire_v1::Part {
        bytes: 1,
        sha256: GUARDED_IMAGE,
    };
    assert!(Image::new(vec![0], &pin).is_err());
    assert!(Image::new(Vec::new(), &pin).is_err());
    for validator in [false, true] {
        metadata(&meta(validator), validator).unwrap();
        for field in 0..10 {
            let mut m = meta(validator);
            match field {
                0 => m.symbol = "old".into(),
                1 => m.object_sha256 = [1; 32],
                2 => m.kernarg_bytes += 8,
                3 => m.kernarg_alignment = 4,
                4 => m.wavefront_size = 32,
                5 => m.private_segment_bytes = 4,
                6 => m.group_segment_bytes = 4,
                7 => m.implicit_argument_offset = Some(0),
                8 => m.implicit_argument_bytes = 0,
                _ => {
                    m.explicit_arguments.pop();
                }
            }
            assert!(metadata(&m, validator).is_err());
        }
        for index in 0..meta(validator).explicit_arguments.len() {
            for field in 0..5 {
                let mut m = meta(validator);
                let a = &mut m.explicit_arguments[index];
                match field {
                    0 => a.offset += 4,
                    1 => a.bytes += 4,
                    2 => a.global_buffer = !a.global_buffer,
                    3 => a.pointee_alignment = Some(4),
                    _ => a.access = Some(BufferAccessV1::Read),
                }
                assert!(metadata(&m, validator).is_err());
            }
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct B {
    id: usize,
    rank: usize,
    bytes: u64,
}
impl Allocation for B {
    fn owner(self) -> usize {
        self.rank
    }
    fn bytes(self) -> u64 {
        self.bytes
    }
}
fn roots() -> LayerBindings<B> {
    let prefix = core::array::from_fn(|r| {
        core::array::from_fn(|i| B {
            id: r * 100 + i,
            rank: r,
            bytes: super::super::PREFIX_BYTES[i],
        })
    });
    let mut mlp = core::array::from_fn(|r| {
        core::array::from_fn(|i| B {
            id: r * 100 + 20 + i,
            rank: r,
            bytes: super::super::MLP_BYTES[i],
        })
    });
    for r in 0..2 {
        mlp[r][5] = prefix[r][7];
        mlp[r][9] = prefix[r][13];
    }
    LayerBindings {
        prefix,
        mlp,
        final_hidden: [prefix[0][0], prefix[1][0]],
    }
}
#[test]
fn guarded_model_down_is_distinct_while_final_output_reuses_exact_own_residual() {
    let roots = roots();
    let down = [
        B {
            id: 200,
            rank: 0,
            bytes: 16384,
        },
        B {
            id: 201,
            rank: 1,
            bytes: 16384,
        },
    ];
    let actual = selected_mlp(&roots, down).unwrap();
    for r in 0..2 {
        assert_eq!(&actual[r][..9], &roots.mlp[r][..9]);
        assert_eq!(actual[r][9], down[r]);
        assert_eq!(roots.final_hidden[r], roots.prefix[r][0]);
        assert_ne!(actual[r][9], roots.prefix[r][13]);
    }
    for r in 0..2 {
        for old in roots
            .prefix
            .iter()
            .flatten()
            .chain(roots.mlp.iter().flatten())
            .chain(roots.final_hidden.iter())
        {
            let mut bad = down;
            bad[r] = *old;
            assert!(selected_mlp(&roots, bad).is_err());
        }
        let mut bad = down;
        bad[r].bytes -= 4;
        assert!(selected_mlp(&roots, bad).is_err());
        let mut bad = down;
        bad[r].rank = 1 - r;
        assert!(selected_mlp(&roots, bad).is_err());
    }
}
fn observation() -> Observation {
    Observation {
        prefixes: [[0; 548]; 2],
        guards: [[1, 0, 1, 0]; 2],
        observed_queue_frontiers: [(5, 3); 2],
        segment_host_ns: 7,
    }
}
struct Fake {
    events: Vec<&'static str>,
    fail: Option<usize>,
    poisoned: bool,
}
impl Fake {
    fn step(&mut self, s: &'static str) -> Result<()> {
        let n = self.events.len();
        self.events.push(s);
        if self.fail == Some(n) {
            Err("injected".into())
        } else {
            Ok(())
        }
    }
}
impl Backend for Fake {
    fn validate(&mut self) -> Result<()> {
        self.step("validate")
    }
    fn prefix(&mut self) -> Result<([[u32; 284]; 2], [u64; 2])> {
        self.step("prefix")?;
        Ok(([[0; 284]; 2], [1, 2]))
    }
    fn guarded(&mut self) -> Result<Observation> {
        self.step("R1/MLP/validator/R2")?;
        Ok(observation())
    }
    fn poison(&mut self) {
        self.poisoned = true;
    }
}
#[test]
fn guarded_model_route_has_one_r2_and_stops_on_every_failed_stage() {
    let mut b = Fake {
        events: Vec::new(),
        fail: None,
        poisoned: false,
    };
    let c = coordinate(&mut b).unwrap();
    assert_eq!(c.prefix_ns, [1, 2]);
    assert_eq!(c.guarded.segment_host_ns, 7);
    assert_eq!(b.events, ["validate", "prefix", "R1/MLP/validator/R2"]);
    assert!(!b.poisoned);
    for fail in 0..3 {
        let mut b = Fake {
            events: Vec::new(),
            fail: Some(fail),
            poisoned: false,
        };
        assert!(coordinate(&mut b).is_err());
        assert!(b.poisoned);
        assert_eq!(b.events.len(), fail + 1);
    }
}
