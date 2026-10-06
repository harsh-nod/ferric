use super::*;

fn metadata(kind: ResidentKind) -> KernelMetadataV1 {
    let spec = kind.spec();
    KernelMetadataV1 {
        symbol: spec.symbol.into(),
        object_sha256: spec.digest,
        kernarg_bytes: spec.explicit_bytes + 256,
        kernarg_alignment: 8,
        group_segment_bytes: spec.lds,
        private_segment_bytes: 0,
        wavefront_size: 64,
        implicit_argument_offset: Some(spec.explicit_bytes),
        implicit_argument_bytes: 256,
        explicit_arguments: (0..spec.arguments).map(|i| argument(kind, i)).collect(),
    }
}

#[test]
fn retained_metadata_has_three_distinct_exact_abis() {
    for kind in ResidentKind::ALL {
        validate_metadata(kind, &metadata(kind)).unwrap();
    }
    assert_eq!(ResidentKind::Prefix.spec().explicit_bytes, 120);
    assert_eq!(ResidentKind::Mlp.spec().explicit_bytes, 88);
    assert_eq!(ResidentKind::Residual.spec().explicit_bytes, 168);
}

#[test]
fn wrong_image_and_resource_metadata_are_rejected() {
    let mutations: [fn(&mut KernelMetadataV1); 11] = [
        |m| m.object_sha256[0] ^= 1,
        |m| m.symbol.push('x'),
        |m| m.kernarg_bytes += 8,
        |m| m.kernarg_alignment = 16,
        |m| m.group_segment_bytes += 4,
        |m| m.private_segment_bytes = 4,
        |m| m.wavefront_size = 32,
        |m| m.implicit_argument_offset = None,
        |m| m.implicit_argument_offset = Some(0),
        |m| m.implicit_argument_bytes = 0,
        |m| {
            m.explicit_arguments.pop();
        },
    ];
    for kind in ResidentKind::ALL {
        for mutate in mutations {
            let mut row = metadata(kind);
            mutate(&mut row);
            assert!(validate_metadata(kind, &row).is_err());
        }
    }
}

#[test]
fn every_argument_offset_width_kind_and_attributes_are_checked() {
    for kind in ResidentKind::ALL {
        for index in 0..kind.spec().arguments {
            for mutation in 0..5 {
                let mut row = metadata(kind);
                let arg = &mut row.explicit_arguments[index];
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
                assert!(
                    validate_metadata(kind, &row).is_err(),
                    "{kind:?}/{index}/{mutation}"
                );
            }
        }
    }
}

#[test]
fn optional_pointer_attributes_do_not_change_exact_image_identity() {
    for kind in ResidentKind::ALL {
        let mut row = metadata(kind);
        for arg in &mut row.explicit_arguments {
            arg.access = None;
            arg.pointee_alignment = None;
        }
        validate_metadata(kind, &row).unwrap();
        row.object_sha256[0] ^= 1;
        assert!(validate_metadata(kind, &row).is_err());
    }
}

#[test]
fn unreviewed_bytes_never_form_a_reviewed_image_set() {
    assert!(ReviewedImages::new(vec![], vec![], vec![]).is_err());
    let bytes = ResidentKind::ALL.map(|kind| vec![0; kind.spec().bytes]);
    let [prefix, mlp, residual] = bytes;
    assert!(ReviewedImages::new(prefix, mlp, residual).is_err());
}

struct Fake {
    events: Vec<(usize, ResidentKind)>,
    failure: Option<usize>,
    corrupt: Option<usize>,
}
impl Loader for Fake {
    type Token = usize;
    fn load(
        &mut self,
        rank: usize,
        kind: ResidentKind,
        _: Vec<u8>,
    ) -> Result<(usize, KernelMetadataV1)> {
        let index = self.events.len();
        self.events.push((rank, kind));
        if self.failure == Some(index) {
            return Err("injected native load failure".into());
        }
        let mut row = metadata(kind);
        if self.corrupt == Some(index) {
            row.object_sha256[0] ^= 1;
        }
        Ok((index, row))
    }
}

fn fixture_images() -> ReviewedImages {
    // Only the private CPU fake bypasses object intake; it cannot open a group.
    ReviewedImages {
        objects: [vec![], vec![], vec![]],
    }
}

#[test]
fn retained_loads_cover_both_ranks_and_all_three_images_once() {
    let mut fake = Fake {
        events: vec![],
        failure: None,
        corrupt: None,
    };
    assert_eq!(
        load_all(&mut fake, fixture_images()).unwrap(),
        [[0, 1, 2], [3, 4, 5]]
    );
    assert_eq!(
        fake.events,
        (0..2)
            .flat_map(|rank| ResidentKind::ALL.map(|kind| (rank, kind)))
            .collect::<Vec<_>>()
    );
}

#[test]
fn each_partial_load_or_metadata_failure_stops_before_the_next_native_operation() {
    for index in 0..6 {
        for corrupt in [false, true] {
            let mut fake = Fake {
                events: vec![],
                failure: (!corrupt).then_some(index),
                corrupt: corrupt.then_some(index),
            };
            assert!(load_all(&mut fake, fixture_images()).is_err());
            assert_eq!(fake.events.len(), index + 1);
        }
    }
}

#[test]
#[ignore = "requires retained P219 object files; run explicitly on the engineering host"]
fn retained_object_bytes_match_the_positive_intake_gate() {
    let read = |name| std::fs::read(std::env::var_os(name).expect("retained object path")).unwrap();
    ReviewedImages::new(
        read("FERRIC_P221_PREFIX_OBJECT"),
        read("FERRIC_P221_MLP_OBJECT"),
        read("FERRIC_P221_RESIDUAL_OBJECT"),
    )
    .unwrap();
}
