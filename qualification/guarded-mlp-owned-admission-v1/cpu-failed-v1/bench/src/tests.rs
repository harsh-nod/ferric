use super::*;

#[test]
fn roster_preserves_two_prefix_then_each_rank_four_stages() {
    let actual = ROLES.map(|r| (r.rank, r.stage, r.image));
    assert_eq!(
        actual,
        [
            (0, "prefix", 0),
            (1, "prefix", 0),
            (0, "r1", 1),
            (0, "mlp", 2),
            (0, "guard", 3),
            (0, "r2", 3),
            (1, "r1", 1),
            (1, "mlp", 2),
            (1, "guard", 3),
            (1, "r2", 3)
        ]
    );
    assert_eq!(ROLES[0].symbol, ROLES[1].symbol);
    for i in 2..6 {
        assert_eq!(ROLES[i].symbol, ROLES[i + 4].symbol);
    }
    assert_ne!(ROLES[4].symbol, ROLES[5].symbol);
}

#[test]
fn multiplicities_and_fixed_samples_are_not_user_tunable() {
    assert_eq!(LAYERS * ROLES.len(), 360);
    let mut per_image = [0; 4];
    for r in ROLES {
        per_image[r.image] += LAYERS;
    }
    assert_eq!(per_image, [72, 72, 72, 144]);
    assert_eq!((WARMUP_PAIRS, SAMPLE_PAIRS), (2, 12));
    assert_eq!(IMAGES.iter().map(|p| p.bytes).sum::<usize>(), 126968);
}

#[test]
fn pair_order_is_balanced_without_timing_dependent_selection() {
    assert_eq!(order(0), [Arm::Fresh, Arm::Owned]);
    assert_eq!(order(1), [Arm::Owned, Arm::Fresh]);
    assert_eq!(
        (0..SAMPLE_PAIRS)
            .filter(|i| order(*i)[0] == Arm::Fresh)
            .count(),
        6
    );
}

#[test]
fn image_authentication_requires_exact_extent_and_original_digest() {
    let p = ImagePin {
        file: "fixture",
        bytes: 3,
        sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    };
    assert!(authenticate_bytes(b"abc", p).is_ok());
    for bytes in [&b"ab"[..], &b"abcd"[..], &b"abd"[..]] {
        assert!(authenticate_bytes(bytes, p).is_err());
    }
    assert!(authenticate_bytes(b"abc", ImagePin { sha256: "00", ..p }).is_err());
}

#[test]
fn fixed_original_pins_are_distinct_bounded_and_lowercase_hex() {
    for (i, p) in IMAGES.iter().enumerate() {
        assert!(p.bytes > 0 && p.bytes < 65536);
        assert_eq!(p.sha256.len(), 64);
        assert!(
            p.sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        );
        for q in &IMAGES[..i] {
            assert_ne!(p.sha256, q.sha256);
            assert_ne!(p.file, q.file);
        }
    }
}

#[test]
fn deadline_refuses_at_boundary_without_budget_extension() {
    let start = Instant::now();
    let until = start + Duration::from_secs(WALL_SECONDS);
    assert!(deadline(start, until).is_ok());
    assert!(deadline(until, until).is_err());
    assert!(deadline(until + Duration::from_nanos(1), until).is_err());
    assert_eq!(REQUIRED_CPU_SECONDS, 45);
    assert_eq!(REQUIRED_ADDRESS_SPACE_BYTES, 256 << 20);
}

#[test]
fn nanoseconds_are_exact_integers_and_overflow_is_refused() {
    assert_eq!(nanos(Duration::from_nanos(u64::MAX)).unwrap(), u64::MAX);
    assert!(nanos(Duration::from_secs(u64::MAX)).is_err());
}

#[test]
fn result_consumption_accounts_for_every_digest_byte_and_refuses_overflow() {
    let mut sum = 0;
    accumulate(&mut sum, [1; 32]).unwrap();
    assert_eq!(sum, 32);
    accumulate(&mut sum, [255; 32]).unwrap();
    assert_eq!(sum, 8192);
    assert!(accumulate(&mut u64::MAX, [1; 32]).is_err());
}

#[test]
fn both_safe_loader_arms_refuse_malformed_object_before_timing() {
    let bytes = vec![0; 64];
    assert!(fresh(&bytes, ROLES[0]).is_err());
    assert!(validate_owned(bytes, PROFILE).is_err());
}

#[test]
fn incomplete_roster_refuses_without_indexing_or_looping() {
    let until = Instant::now() + Duration::from_secs(1);
    assert!(sample(Arm::Fresh, &[], &[], until).is_err());
    assert!(sample(Arm::Owned, &[], &[], until).is_err());
    assert!(check_roles(&[], &[], until).is_err());
}
