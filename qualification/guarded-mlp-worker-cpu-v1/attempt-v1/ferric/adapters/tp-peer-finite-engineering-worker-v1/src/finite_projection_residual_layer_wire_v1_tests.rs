use super::*;
fn bootstrap() -> Bootstrap {
    Bootstrap {
        schema: SCHEMA.into(),
        layer: base::tests::bootstrap(base::Profile::Prefix284Mlp548),
        projection_residual_image: part(&[2]),
    }
}
#[test]
fn projection_bootstrap_roundtrip_preserves_original_copy_image() {
    let b = bootstrap();
    let old = b.layer.begin.residual_image;
    let mut raw = Vec::new();
    write_bootstrap(&mut raw, &mut Budget::new(), &b, &[1], &[1], &[2]).unwrap();
    let (decoded, mlp, prefix, candidate) =
        read_bootstrap(&mut raw.as_slice(), &mut Budget::new()).unwrap();
    assert_eq!(decoded, b);
    assert_eq!((mlp, prefix, candidate), (vec![1], vec![1], vec![2]));
    assert_eq!(decoded.layer.begin.residual_image, old);
    assert_ne!(old, decoded.projection_residual_image);
    let last = raw.len() - 1;
    raw[last] ^= 1;
    assert!(read_bootstrap(&mut raw.as_slice(), &mut Budget::new()).is_err());
}
#[test]
fn projection_bootstrap_rejects_profile_schema_bounds_and_extra_fields() {
    for field in 0..6 {
        let mut b = bootstrap();
        match field {
            0 => b.schema.push('x'),
            1 => {
                b.layer.profile = base::Profile::Baseline22Mlp548;
                b.layer.prefix_image = None;
            }
            2 => b.projection_residual_image.bytes = 0,
            3 => b.projection_residual_image.bytes = (32 << 20) + 1,
            4 => b.projection_residual_image.sha256 = [0; 32],
            _ => b.layer.begin.tail_image = None,
        }
        assert!(b.validate().is_err());
    }
    let mut value = serde_json::to_value(bootstrap()).unwrap();
    value["numerical_acceptance"] = true.into();
    assert!(serde_json::from_value::<Bootstrap>(value).is_err());
}
#[test]
fn projection_profile_joins_both_identities_and_cannot_alias_old_profile() {
    let b = bootstrap();
    let old = b.layer.sha256().unwrap();
    let sha = b.sha256().unwrap();
    assert_ne!(old, sha);
    assert_eq!(
        sha,
        profile_sha256(old, b.projection_residual_image.sha256).unwrap()
    );
    assert_ne!(sha, profile_sha256(old, [9; 32]).unwrap());
    assert_ne!(
        sha,
        profile_sha256([9; 32], b.projection_residual_image.sha256).unwrap()
    );
    assert!(profile_sha256(old, [0; 32]).is_err());
    assert!(profile_sha256([0; 32], [1; 32]).is_err());
}
#[test]
fn projection_begin_and_run_close_fit_original_closed_budget() {
    let b = bootstrap();
    let mut raw = Vec::new();
    write_bootstrap(&mut raw, &mut Budget::new(), &b, &[1], &[1], &[2]).unwrap();
    let begin = crate::finite_setup_wire_v1::Request {
        protocol: 1,
        id: 1,
        device_ids: b.layer.device_ids,
        session: b.layer.begin.scope.session,
        command: crate::finite_setup_wire_v1::Command::Begin(b.layer.begin.clone()),
    };
    crate::finite_setup_wire_v1::write_request(&mut raw, &begin, &[1; 7]).unwrap();
    let sha = b.sha256().unwrap();
    for (id, command) in [(1, Command::Run), (2, Command::Close)] {
        write_request(
            &mut raw,
            &mut Budget::new(),
            &Request {
                protocol: 1,
                id,
                profile_sha256: sha,
                command,
            },
        )
        .unwrap();
    }
    let mut read = raw.as_slice();
    let mut budget = Budget::new();
    read_bootstrap(&mut read, &mut budget).unwrap();
    assert_eq!(
        read_begin(&mut read, &mut budget, &b).unwrap(),
        (begin, vec![1; 7])
    );
    assert_eq!(read_request(&mut read, &mut budget).unwrap().id, 1);
    assert_eq!(read_request(&mut read, &mut budget).unwrap().id, 2);
    assert!(read.is_empty());
}
