use super::*;
use crate::finite_guarded_mlp_long_wire_v2::tests as fixture;

fn bootstrap() -> Bootstrap {
    Bootstrap {
        schema: SCHEMA.into(),
        child_deadline_ms: 60_000,
        sequence: fixture::bootstrap(long::Profile::Readiness40),
    }
}
#[test]
fn readiness_wire_profile_is_native40_only_and_setup_keeps_ar4_closed() {
    let b = bootstrap();
    let old = b.setup().unwrap();
    assert_eq!(old.schema, four::REUSE_SCHEMA);
    assert_eq!(old.decode.input_tokens, vec![b.sequence.prompt_tokens[0]]);
    assert_ne!(old.sha256().unwrap(), b.sequence.sha256().unwrap());
    assert_eq!(four::FORWARDS, 4);
    assert!(fixture::control(5).validate(5).is_err());
    for edit in [
        |v: &mut Bootstrap| v.schema = four::REUSE_SCHEMA.into(),
        |v: &mut Bootstrap| v.sequence.profile = long::Profile::Full2303,
        |v: &mut Bootstrap| v.child_deadline_ms = 999,
        |v: &mut Bootstrap| v.child_deadline_ms = MAX_DEADLINE_MS + 1,
        |v: &mut Bootstrap| v.sequence.prompt_tokens.truncate(40),
    ] {
        let mut bad = b.clone();
        edit(&mut bad);
        assert!(bad.setup().is_err());
    }
}
#[test]
fn readiness_wire_close_requires_forty_zero_outputs_and_exact_identity() {
    let b = bootstrap();
    let request = fixture::close_request(&b.sequence);
    let good = Closed::new(request.clone(), [7; 32]).unwrap();
    good.validate(&request, [7; 32]).unwrap();
    for edit in [
        |v: &mut Closed| v.completed_forwards = 4,
        |v: &mut Closed| v.generated_tokens.push(7),
        |v: &mut Closed| v.native_closed = false,
        |v: &mut Closed| v.performance_claim = true,
        |v: &mut Closed| v.numerical_acceptance = true,
        |v: &mut Closed| v.production_authority = true,
        |v: &mut Closed| v.request.session[0] ^= 1,
        |v: &mut Closed| v.transcript_sha256[0] ^= 1,
    ] {
        let mut bad = good.clone();
        edit(&mut bad);
        assert!(bad.validate(&request, [7; 32]).is_err());
    }
    assert!(Closed::new(fixture::request(&b.sequence, 0, 0), [7; 32]).is_err());
    assert!(
        Closed::new(
            fixture::close_request(&fixture::bootstrap(long::Profile::Full2303)),
            [7; 32]
        )
        .is_err()
    );
}
#[test]
fn readiness_wire_refuses_unknown_fields_truncation_and_unbound_images() {
    let b = bootstrap();
    let mut value = serde_json::to_value(&b).unwrap();
    value["full2303"] = true.into();
    assert!(serde_json::from_value::<Bootstrap>(value).is_err());
    let mut raw = Vec::new();
    write_header(&mut raw, &mut long::FrameBudget::new(), &b).unwrap();
    assert!(read_bootstrap(&mut raw.as_slice(), &mut long::FrameBudget::new()).is_err());
    let mut output = Vec::new();
    assert!(write_bootstrap(&mut output, &mut long::FrameBudget::new(), &b, [&[1]; 4]).is_err());
    assert!(output.is_empty());
}

#[test]
fn position5_wire_requires_matching_schema_entry_and_close() {
    let mut b = bootstrap();
    b.sequence.profile = long::Profile::Readiness40Position5;
    assert!(b.setup().is_err());
    b.schema = POSITION5_SCHEMA.into();
    b.setup().unwrap();
    let mut raw = Vec::new();
    write_header(&mut raw, &mut long::FrameBudget::new(), &b).unwrap();
    let error = read_bootstrap(&mut raw.as_slice(), &mut long::FrameBudget::new()).unwrap_err();
    assert_eq!(
        error.to_string(),
        "readiness explicit entry/profile mismatch"
    );
    let request = fixture::close_request(&b.sequence);
    let close = Closed::new_for(b.sequence.profile, request.clone(), [7; 32]).unwrap();
    close
        .validate_for(b.sequence.profile, &request, [7; 32])
        .unwrap();
    assert!(close.validate(&request, [7; 32]).is_err());
    assert!(schemas(long::Profile::Full2303).is_err());
}
