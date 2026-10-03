use super::*;
pub(crate) fn bootstrap(profile: Profile) -> Bootstrap {
    let p = part(&[1]);
    Bootstrap {
        protocol: 1,
        profile,
        device_ids: [7, 9],
        timeout_ms: 1000,
        begin: Begin {
            scope: setup::Scope {
                bundle_id: [1; 32],
                model_id: [2; 32],
                session: [3; 32],
                pool_identity: 1,
                group_id: 0,
                child_identity: std::process::id(),
            },
            registration: p,
            source_program: p,
            uploads: p,
            prefix_image: p,
            mlp_image: p,
            residual_image: p,
            tail_image: Some(p),
        },
        input: Input {
            generation: 1,
            token: 9112,
            cache_metadata: core::iter::once(0).chain(0..144).collect(),
            rotary_bits: vec![0; 128],
        },
        mlp_image: p,
        prefix_image: if profile == Profile::Prefix284Mlp548 {
            Some(p)
        } else {
            None
        },
    }
}
pub(crate) fn control(profile: Profile) -> Control {
    let prefix = match profile {
        Profile::Baseline22Mlp548 => {
            let mut p = vec![64; 22];
            p[..6].copy_from_slice(&[1, 0, 65535, 65535, 0x55555555, 0]);
            p
        }
        Profile::Prefix284Mlp548 => {
            let mut p = vec![0; 284];
            p[..4].copy_from_slice(&[1, 0, 0, 31]);
            p[4..9].copy_from_slice(&[1, 48, 1, 16, 64]);
            p[9..14].copy_from_slice(&[1, 48, 1, 16, 64]);
            p[14..19].copy_from_slice(&[u32::MAX, u32::MAX, u32::MAX, u32::MAX, 3]);
            p[19..24].copy_from_slice(&[u32::MAX, u32::MAX, u32::MAX, u32::MAX, 3]);
            p[24..154].fill(1);
            p[154..].fill(64);
            p
        }
    };
    let mut mlp = vec![0; 548];
    mlp[..4].copy_from_slice(&[1, 0, 0, 31]);
    mlp[4..9].copy_from_slice(&[1, 96, 96, 1, 64]);
    mlp[9..14].copy_from_slice(&[1, 96, 96, 1, 64]);
    mlp[14..23].fill(u32::MAX);
    mlp[22] = 3;
    mlp[23..32].fill(u32::MAX);
    mlp[31] = 3;
    mlp[32..290].fill(1);
    mlp[290..].fill(64);
    Control {
        prefix: [prefix.clone(), prefix],
        mlp: [mlp.clone(), mlp],
        embedding_ns: [0; 2],
        paired_ns: [[0; 2]; 4],
    }
}
#[test]
fn distinct_bootstraps_roundtrip_and_body_pins() {
    let mut hashes = Vec::new();
    for profile in [Profile::Baseline22Mlp548, Profile::Prefix284Mlp548] {
        let b = bootstrap(profile);
        let image = [1_u8];
        let prefix = b.prefix_image.map(|_| image.as_slice());
        let mut raw = Vec::new();
        write_bootstrap(&mut raw, &mut Budget::new(), &b, &[1], prefix).unwrap();
        assert_eq!(
            read_bootstrap(&mut raw.as_slice(), &mut Budget::new()).unwrap(),
            (b.clone(), vec![1], prefix.map(<[u8]>::to_vec))
        );
        let last = raw.len() - 1;
        raw[last] ^= 1;
        assert!(read_bootstrap(&mut raw.as_slice(), &mut Budget::new()).is_err());
        hashes.push(b.sha256().unwrap());
    }
    assert_ne!(hashes[0], hashes[1]);
}
#[test]
fn bootstrap_and_actual_input_closed_bounds() {
    let b = bootstrap(Profile::Prefix284Mlp548);
    for which in 0..10 {
        let mut bad = b.clone();
        match which {
            0 => bad.begin.tail_image = None,
            1 => bad.prefix_image = None,
            2 => bad.input.generation = 2,
            3 => bad.input.cache_metadata[0] = 1,
            4 => bad.input.cache_metadata[144] = 0,
            5 => bad.input.rotary_bits[0] = f32::NAN.to_bits(),
            6 => bad.begin.registration.bytes = (4 << 20) + 1,
            7 => bad.mlp_image.bytes = 64 << 20,
            8 => bad.device_ids = [7, 7],
            _ => bad.begin.source_program.sha256 = [0; 32],
        }
        assert!(bad.validate().is_err());
    }
    let mut raw = serde_json::to_value(&b).unwrap();
    raw["kernel_admission"] = "cached_immutable".into();
    assert!(serde_json::from_value::<Bootstrap>(raw).is_err());
}
#[test]
fn begin_is_joined_before_any_payload_allocation() {
    let b = bootstrap(Profile::Prefix284Mlp548);
    let good = setup::Request {
        protocol: setup::PROTOCOL,
        id: 1,
        device_ids: b.device_ids,
        session: b.begin.scope.session,
        command: setup::Command::Begin(b.begin.clone()),
    };
    let mut raw = Vec::new();
    setup::write_request(&mut raw, &good, &[1; 7]).unwrap();
    assert_eq!(
        read_begin(&mut raw.as_slice(), &mut Budget::new(), &b).unwrap(),
        (good.clone(), vec![1; 7])
    );
    let mut bad = good;
    bad.id = 2;
    let mut raw = Vec::new();
    write_header(&mut raw, &mut Budget::new(), &bad, 0).unwrap();
    // There is intentionally no payload: the exact header fails first.
    let error = read_begin(&mut raw.as_slice(), &mut Budget::new(), &b).unwrap_err();
    assert!(error.to_string().contains("exact bootstrap Begin"));
}
#[test]
fn every_terminal_word_is_validated_and_types_do_not_alias() {
    for profile in [Profile::Baseline22Mlp548, Profile::Prefix284Mlp548] {
        let c = control(profile);
        c.validate(profile).unwrap();
        assert!(
            c.validate(if profile == Profile::Baseline22Mlp548 {
                Profile::Prefix284Mlp548
            } else {
                Profile::Baseline22Mlp548
            })
            .is_err()
        );
        for rank in 0..2 {
            for i in 0..c.prefix[rank].len() {
                let mut bad = c.clone();
                bad.prefix[rank][i] = if c.prefix[rank][i] == 0 { 1 } else { 0 };
                assert!(bad.validate(profile).is_err(), "prefix {rank}/{i}");
            }
            for i in 0..548 {
                let mut bad = c.clone();
                bad.mlp[rank][i] = if c.mlp[rank][i] == 0 { 1 } else { 0 };
                assert!(bad.validate(profile).is_err(), "MLP {rank}/{i}");
            }
        }
    }
}
#[test]
fn exact_capture_geometry_and_finite_widths() {
    assert_eq!(
        STAGES.iter().map(|(_, n, _)| n).sum::<usize>() * 2,
        CAPTURE_BYTES
    );
    let raw = vec![0; CAPTURE_BYTES];
    assert_eq!(capture_rows(&raw).unwrap().len(), 28);
    assert!(capture_rows(&raw[..raw.len() - 1]).is_err());
    let mut offset = 0;
    for _ in 0..2 {
        for (_, n, width) in STAGES {
            let mut bad = raw.clone();
            if width == 2 {
                bad[offset..offset + 2].copy_from_slice(&0x7f80_u16.to_le_bytes());
            } else {
                bad[offset..offset + 4].copy_from_slice(&f32::NAN.to_bits().to_le_bytes());
            }
            assert!(capture_rows(&bad).is_err());
            offset += n;
        }
    }
}
#[test]
fn only_closed_success_releases_capture() {
    for profile in [Profile::Baseline22Mlp548, Profile::Prefix284Mlp548] {
        let b = bootstrap(profile);
        let raw = vec![0; CAPTURE_BYTES];
        let mut r = Response {
            protocol: 1,
            id: 2,
            profile_sha256: b.sha256().unwrap(),
            profile,
            native_closed: true,
            completed_layers: 1,
            control: Some(control(profile)),
            capture: Some(part(&raw)),
            gpu_execution: true,
            numerical_acceptance: false,
            performance_claim: false,
            production_authority: false,
        };
        let mut encoded = Vec::new();
        write_response(&mut encoded, &mut Budget::new(), &r, &raw).unwrap();
        assert_eq!(
            read_response(&mut encoded.as_slice(), &mut Budget::new()).unwrap(),
            (r.clone(), raw)
        );
        r.native_closed = false;
        assert!(r.validate().is_err());
        r.native_closed = true;
        r.numerical_acceptance = true;
        assert!(r.validate().is_err());
        r.numerical_acceptance = false;
        r.id = 1;
        assert!(r.validate().is_err());
        r.control = None;
        r.capture = None;
        r.native_closed = false;
        r.validate().unwrap();
    }
}
#[test]
fn frame_and_header_caps_fail_closed() {
    let mut budget = Budget::new();
    budget.add(LIMIT).unwrap();
    assert!(budget.add(1).is_err());
    let mut budget = Budget::new();
    for _ in 0..8 {
        budget.add(1).unwrap();
    }
    assert!(budget.add(1).is_err());
    let mut budget = Budget {
        bytes: usize::MAX,
        frames: 0,
    };
    assert!(budget.add(1).is_err());
    assert!(
        read_header::<Request>(
            &mut ((HEADER_LIMIT + 1) as u32).to_le_bytes().as_slice(),
            &mut Budget::new()
        )
        .is_err()
    );
    assert!(header(&vec![0_u8; HEADER_LIMIT]).is_err());
}
#[test]
fn request_sequence_is_exact_and_rejects_extra_fields() {
    for (id, command) in [(1, Command::Run), (2, Command::Close)] {
        let r = Request {
            protocol: 1,
            id,
            profile_sha256: [1; 32],
            command,
        };
        r.validate().unwrap();
        let mut bad = r.clone();
        bad.id = 3 - id;
        assert!(bad.validate().is_err());
        let mut json = serde_json::to_value(&r).unwrap();
        json["generation"] = 2.into();
        assert!(serde_json::from_value::<Request>(json).is_err());
    }
}
