use super::*;
use crate::finite_guarded_mlp_long_wire_v2::tests as fixture;
fn bootstrap() -> Bootstrap {
    Bootstrap {
        schema: SCHEMA.into(),
        child_deadline_ms: 60_000,
        sequence: fixture::bootstrap(long::Profile::Full2303),
    }
}
#[test]
fn full2303_wire_is_separate_and_keeps_original_setup_and_caps() {
    let b = bootstrap();
    let setup = b.setup().unwrap();
    assert_eq!(setup.schema, four::REUSE_SCHEMA);
    assert_eq!(setup.decode.input_tokens, vec![b.sequence.prompt_tokens[0]]);
    assert_ne!(setup.sha256().unwrap(), b.sequence.sha256().unwrap());
    assert_eq!(MAX_DEADLINE_MS, 3_600_000);
    assert_eq!(
        crate::finite_guarded_mlp_readiness_wire_v1::MAX_DEADLINE_MS,
        3_600_000
    );
    assert_eq!(four::FORWARDS, 4);
    for edit in [
        |v: &mut Bootstrap| v.schema = four::REUSE_SCHEMA.into(),
        |v: &mut Bootstrap| v.sequence.profile = long::Profile::Readiness40,
        |v: &mut Bootstrap| v.sequence.profile = long::Profile::Readiness40Position5,
        |v: &mut Bootstrap| v.child_deadline_ms = 999,
        |v: &mut Bootstrap| v.child_deadline_ms = MAX_DEADLINE_MS + 1,
        |v: &mut Bootstrap| v.sequence.prompt_tokens.truncate(40),
    ] {
        let mut bad = b.clone();
        edit(&mut bad);
        assert!(bad.setup().is_err());
    }
    assert!(crate::finite_guarded_mlp_readiness_wire_v1::schemas(long::Profile::Full2303).is_err());
}
#[test]
fn full2303_close_requires_all_actual_generated_ids_and_false_authority() {
    let b = bootstrap();
    let request = fixture::close_request(&b.sequence);
    let tokens: Vec<u32> = (0..256).collect();
    let good = Closed {
        schema: CLOSE_SCHEMA.into(),
        request: request.clone(),
        completed_forwards: 2303,
        generated_tokens: tokens.clone(),
        transcript_sha256: [7; 32],
        native_closed: true,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
    };
    good.validate(&request, [7; 32], &tokens).unwrap();
    for edit in [
        |v: &mut Closed| v.completed_forwards = 40,
        |v: &mut Closed| {
            v.generated_tokens.pop();
        },
        |v: &mut Closed| v.generated_tokens[255] ^= 1,
        |v: &mut Closed| v.generated_tokens[0] = 151_936,
        |v: &mut Closed| v.native_closed = false,
        |v: &mut Closed| v.numerical_acceptance = true,
        |v: &mut Closed| v.performance_claim = true,
        |v: &mut Closed| v.production_authority = true,
        |v: &mut Closed| v.request.session[0] ^= 1,
        |v: &mut Closed| v.transcript_sha256[0] ^= 1,
    ] {
        let mut bad = good.clone();
        edit(&mut bad);
        assert!(bad.validate(&request, [7; 32], &tokens).is_err());
    }
    let mut other = tokens.clone();
    other[0] ^= 1;
    assert!(good.validate(&request, [7; 32], &other).is_err());
    assert!(good.validate(&request, [7; 32], &tokens[..255]).is_err());
    assert!(
        good.validate(&fixture::request(&b.sequence, 0, 0), [7; 32], &tokens)
            .is_err()
    );
}
#[test]
fn full2303_close_factory_refuses_unclosed_and_readiness_transcripts() {
    for profile in [
        long::Profile::Full2303,
        long::Profile::Readiness40,
        long::Profile::Readiness40Position5,
    ] {
        let b = fixture::bootstrap(profile);
        let t = long::Transcript::new(b.clone()).unwrap();
        assert!(Closed::from_transcript(fixture::close_request(&b), &t).is_err());
    }
}
#[test]
fn full2303_wire_refuses_unknown_fields_truncated_and_unpinned_images() {
    let b = bootstrap();
    let mut value = serde_json::to_value(&b).unwrap();
    value["allow_longer_deadline"] = true.into();
    assert!(serde_json::from_value::<Bootstrap>(value).is_err());
    let mut raw = Vec::new();
    write_header(&mut raw, &mut long::FrameBudget::new(), &b).unwrap();
    assert!(read_bootstrap(&mut raw.as_slice(), &mut long::FrameBudget::new()).is_err());
    let mut output = Vec::new();
    assert!(write_bootstrap(&mut output, &mut long::FrameBudget::new(), &b, [&[1]; 4]).is_err());
    assert!(output.is_empty());
}
