use super::*;

fn metadata(kind: TailKind) -> KernelMetadataV1 {
    KernelMetadataV1 {
        symbol: kind.symbol().into(),
        object_sha256: IMAGE_SHA[kind.image()],
        kernarg_bytes: kind.explicit_bytes() + 256,
        kernarg_alignment: 8,
        group_segment_bytes: 0,
        private_segment_bytes: 0,
        wavefront_size: 64,
        implicit_argument_offset: Some(kind.explicit_bytes()),
        implicit_argument_bytes: 256,
        explicit_arguments: arguments(kind),
    }
}

#[test]
fn five_closed_tail_entries_have_exact_rank_grid_and_abi() {
    let rows = [
        (TailKind::Embedding, 0, 4096, 312, 7),
        (TailKind::Copy, 1, 4096, 296, 5),
        (TailKind::FinalNorm, 0, 64, 352, 14),
        (TailKind::Head, 0, 607744, 328, 11),
        (TailKind::Argmax, 0, 64, 296, 5),
    ];
    for (kind, rank, grid, bytes, count) in rows {
        validate_metadata(kind, &metadata(kind)).unwrap();
        assert_eq!(kind.rank(), rank);
        assert_eq!(kind.grid(), [grid, 1, 1]);
        assert_eq!(kind.explicit_bytes() + 256, bytes);
        assert_eq!(arguments(kind).len(), count);
    }
}

#[test]
fn tail_metadata_every_identity_resource_and_argument_is_exact() {
    let mutations: [fn(&mut KernelMetadataV1); 10] = [
        |m| m.object_sha256[0] ^= 1,
        |m| m.symbol.push('x'),
        |m| m.kernarg_bytes += 8,
        |m| m.kernarg_alignment = 16,
        |m| m.group_segment_bytes = 4,
        |m| m.private_segment_bytes = 4,
        |m| m.wavefront_size = 32,
        |m| m.implicit_argument_offset = Some(0),
        |m| m.implicit_argument_bytes = 0,
        |m| {
            m.explicit_arguments.pop();
        },
    ];
    for kind in TailKind::ALL {
        for mutation in mutations {
            let mut m = metadata(kind);
            mutation(&mut m);
            assert!(validate_metadata(kind, &m).is_err());
        }
        for index in 0..arguments(kind).len() {
            for mutation in 0..5 {
                let mut m = metadata(kind);
                let arg = &mut m.explicit_arguments[index];
                match mutation {
                    0 => arg.offset += 4,
                    1 => arg.bytes += 4,
                    2 => arg.global_buffer = !arg.global_buffer,
                    3 => arg.pointee_alignment = Some(16),
                    _ => {
                        arg.access = Some(if arg.access == Some(Access::Read) {
                            Access::Write
                        } else {
                            Access::Read
                        })
                    }
                }
                assert!(validate_metadata(kind, &m).is_err());
            }
        }
    }
}

#[test]
fn tail_optional_attributes_do_not_allow_another_image() {
    for kind in TailKind::ALL {
        let mut m = metadata(kind);
        for arg in &mut m.explicit_arguments {
            arg.access = None;
            arg.pointee_alignment = None;
        }
        validate_metadata(kind, &m).unwrap();
        m.object_sha256[0] ^= 1;
        assert!(validate_metadata(kind, &m).is_err());
    }
    assert!(ReviewedTailImages::new(vec![0; IMAGE_BYTES[0]], vec![0; IMAGE_BYTES[1]]).is_err());
}

struct Fake {
    events: Vec<TailKind>,
    fail: Option<usize>,
    corrupt: Option<usize>,
}
impl Loader for Fake {
    type Token = usize;
    fn load(&mut self, kind: TailKind, _: Vec<u8>) -> Result<(usize, KernelMetadataV1)> {
        let index = self.events.len();
        self.events.push(kind);
        if self.fail == Some(index) {
            return Err("injected tail load failure".into());
        }
        let mut m = metadata(kind);
        if self.corrupt == Some(index) {
            m.object_sha256[0] ^= 1;
        }
        Ok((index, m))
    }
}

#[test]
fn tail_loader_stops_after_every_partial_failure_without_fallback() {
    let images = || ReviewedTailImages {
        objects: [vec![], vec![]],
    };
    let mut fake = Fake {
        events: vec![],
        fail: None,
        corrupt: None,
    };
    assert_eq!(load_all(&mut fake, images()).unwrap(), [0, 1, 2, 3, 4]);
    assert_eq!(fake.events, TailKind::ALL);
    for index in 0..5 {
        for corrupt in [false, true] {
            let mut fake = Fake {
                events: vec![],
                fail: (!corrupt).then_some(index),
                corrupt: corrupt.then_some(index),
            };
            assert!(load_all(&mut fake, images()).is_err());
            assert_eq!(fake.events.len(), index + 1);
        }
    }
}

#[test]
#[ignore = "requires exact retained V3 and V18 object paths; CPU intake only"]
fn retained_tail_image_bytes_pass_exact_intake() {
    let read = |key| std::fs::read(std::env::var_os(key).expect("retained image path")).unwrap();
    ReviewedTailImages::new(
        read("FERRIC_P222_V3_OBJECT"),
        read("FERRIC_P222_V18_OBJECT"),
    )
    .unwrap();
}
