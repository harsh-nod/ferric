use super::*;
use crate::finite_guarded_mlp_long_wire_v2::tests as fixture;

fn bootstrap() -> Bootstrap {
    let mut b = fixture::bootstrap(Profile::Readiness40);
    b.scope.child_identity = std::process::id();
    b.begin.scope = b.scope.clone();
    b
}
fn original(b: &Bootstrap) -> super::super::Profile {
    super::super::Profile::new(
        &b.scope,
        b.registration,
        b.prefix_image.sha256,
        b.mlp_image.sha256,
        b.projection_image.sha256,
        Mode::Autoregressive {
            first: b.prompt_tokens[0],
        },
        b.timeout_ms,
        b.device_ids,
    )
    .unwrap()
    .with_reusable_arenas()
    .unwrap()
}

#[test]
fn readiness_profile_binds_actual_reusable_owner_and_rejects_full_workload() {
    let b = bootstrap();
    let p = original(&b);
    profile_join(&b, &p).unwrap();
    for case in 0..9 {
        let mut bad = b.clone();
        match case {
            0 => bad.profile = Profile::Full2303,
            1 => bad.scope.session[0] ^= 1,
            2 => bad.registration[0] ^= 1,
            3 => bad.prefix_image.sha256[0] ^= 1,
            4 => bad.mlp_image.sha256[0] ^= 1,
            5 => bad.projection_image.sha256[0] ^= 1,
            6 => bad.device_ids.swap(0, 1),
            7 => bad.timeout_ms += 1,
            _ => bad.prompt_tokens[0] += 1,
        }
        assert!(profile_join(&bad, &p).is_err(), "case {case}");
    }
    let mut fresh = original(&b);
    fresh.reusable_arenas = false;
    assert!(profile_join(&b, &fresh).is_err());
}

#[test]
fn readiness_native_input_keeps_all_144_pages_and_crosses_two_page_boundaries() {
    let b = bootstrap();
    for position in [0, 15, 16, 31, 32, 39] {
        let r = fixture::request(&b, position, 0);
        let i = input(&r).unwrap();
        assert_eq!(i.generation, u64::from(position) + 1);
        assert_eq!(i.cache_metadata[0], position);
        assert_eq!(i.cache_metadata[1..], (0..144).collect::<Vec<_>>());
        assert_eq!(i.rotary_bits, [0; 128]);
    }
    let mut r = fixture::request(&b, 0, 0);
    let Command::Forward { cache_metadata, .. } = &mut r.command else {
        unreachable!()
    };
    cache_metadata[144] = 0;
    assert!(input(&r).is_err());
    let mut r = fixture::request(&b, 0, 0);
    let Command::Forward { rotary_bits, .. } = &mut r.command else {
        unreachable!()
    };
    rotary_bits[127] = f32::NAN.to_bits();
    assert!(input(&r).is_err());
    let mut r = fixture::request(&b, 0, 0);
    r.command = Command::Close;
    r.id = 41;
    assert!(input(&r).is_err());
    assert!(input(&fixture::request(&b, 40, 0)).is_err());
}

#[test]
fn readiness_cache_shapes_require_full_context_not_four_or_forty_tokens() {
    let rows = (0..144)
        .map(|i| {
            (
                (i / 72) as u32,
                (i / 2 % 36) as u32,
                i % 2,
                i as u64 + 1,
                CACHE_BYTES,
                CACHE_BYTES,
            )
        })
        .collect::<Vec<_>>();
    cache_shapes(&rows).unwrap();
    for index in [0, 71, 72, 143] {
        for case in 0..7 {
            let mut bad = rows.clone();
            match case {
                0 => bad[index].0 ^= 1,
                1 => bad[index].1 = 36,
                2 => bad[index].2 = 2,
                3 => bad[index].3 = 0,
                4 => bad[index].4 = 4 * 512 * 2,
                5 => bad[index].4 = 40 * 512 * 2,
                _ => bad[index].5 -= 2,
            }
            assert!(cache_shapes(&bad).is_err(), "row {index} case {case}");
        }
    }
    let mut alias = rows.clone();
    alias[1].3 = alias[0].3;
    assert!(cache_shapes(&alias).is_err());
    assert!(cache_shapes(&rows[..143]).is_err());
    let mut extra = rows.clone();
    extra.push(rows[0]);
    assert!(cache_shapes(&extra).is_err());
}

#[test]
fn readiness_deadline_is_fixed_and_cutoff_is_not_extended() {
    let now = Instant::now();
    let deadline = now + Duration::from_secs(1);
    remaining(now, deadline).unwrap();
    admit_deadline(now, now + WHOLE_LIMIT).unwrap();
    assert!(admit_deadline(now, now + WHOLE_LIMIT + Duration::from_nanos(1)).is_err());
    assert!(admit_deadline(deadline, deadline).is_err());
    remaining(deadline - Duration::from_nanos(1), deadline).unwrap();
    assert!(remaining(deadline, deadline).is_err());
    assert!(remaining(deadline + Duration::from_nanos(1), deadline).is_err());
    assert_eq!(WHOLE_LIMIT, Duration::from_secs(3600));
}

#[test]
fn position5_owner_profile_is_explicit_and_keeps_same_native_inputs() {
    let mut b = bootstrap();
    let original_profile = original(&b);
    b.profile = Profile::Readiness40Position5;
    assert!(profile_join(&b, &original_profile).is_err());
    profile_join_for(&b, &original_profile, Profile::Readiness40Position5).unwrap();
    for position in 0..40 {
        let request = fixture::request(&b, position, 0);
        let actual = input_for(&request, b.profile).unwrap();
        assert_eq!(actual.cache_metadata[0], position);
        assert_eq!(actual.token, b.prompt_tokens[position as usize]);
        assert_eq!(actual.generation, u64::from(position) + 1);
    }
    b.profile = Profile::Readiness40;
    assert!(profile_join_for(&b, &original_profile, Profile::Readiness40Position5).is_err());
}
