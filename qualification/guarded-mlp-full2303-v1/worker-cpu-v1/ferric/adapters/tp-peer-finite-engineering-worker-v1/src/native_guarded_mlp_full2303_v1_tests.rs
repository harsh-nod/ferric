use super::*;
use crate::finite_guarded_mlp_long_wire_v2::tests as fixture;
fn bootstrap() -> Bootstrap {
    let mut b = fixture::bootstrap(Profile::Full2303);
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
fn full2303_owner_binds_original_pristine_reusable_profile_without_other_modes() {
    let b = bootstrap();
    let p = original(&b);
    profile_join(&b, &p).unwrap();
    for case in 0..10 {
        let mut bad = b.clone();
        match case {
            0 => bad.profile = Profile::Readiness40,
            1 => bad.profile = Profile::Readiness40Position5,
            2 => bad.scope.session[0] ^= 1,
            3 => bad.registration[0] ^= 1,
            4 => bad.prefix_image.sha256[0] ^= 1,
            5 => bad.mlp_image.sha256[0] ^= 1,
            6 => bad.projection_image.sha256[0] ^= 1,
            7 => bad.device_ids.swap(0, 1),
            8 => bad.timeout_ms += 1,
            _ => bad.prompt_tokens[0] += 1,
        }
        assert!(profile_join(&bad, &p).is_err(), "case {case}");
    }
    let mut fresh = original(&b);
    fresh.reusable_arenas = false;
    assert!(profile_join(&b, &fresh).is_err());
    let paired = original(&b).with_paired_terminal().unwrap();
    assert!(profile_join(&b, &paired).is_err());
}
#[test]
fn full2303_owner_deadline_is_an_unchanged_absolute_abort_bound() {
    let now = Instant::now();
    assert_eq!(WHOLE_LIMIT, Duration::from_secs(3600));
    admit_deadline(now, now + WHOLE_LIMIT).unwrap();
    assert!(admit_deadline(now, now + WHOLE_LIMIT + Duration::from_nanos(1)).is_err());
    assert!(admit_deadline(now, now).is_err());
    assert!(admit_deadline(now + Duration::from_secs(1), now).is_err());
}
