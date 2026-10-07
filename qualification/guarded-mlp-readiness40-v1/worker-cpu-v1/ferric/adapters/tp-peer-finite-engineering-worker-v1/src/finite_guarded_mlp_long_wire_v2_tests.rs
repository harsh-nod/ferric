use super::*;
use crate::finite_guarded_mlp_decode_wire_v1 as four;

pub(crate) fn bootstrap(profile: Profile) -> Bootstrap {
    let b = four::tests::bootstrap(four::InputMode::Autoregressive);
    Bootstrap {
        protocol: 1,
        profile,
        device_ids: b.decode.device_ids,
        scope: b.decode.scope.clone(),
        registration: b.decode.registration,
        begin: b.decode.begin.clone(),
        timeout_ms: b.decode.timeout_ms,
        prompt_tokens: (0..PROMPT_TOKENS).map(|i| i as u32 % 1000 + 2).collect(),
        prefix_image: b.decode.prefix_image,
        mlp_image: b.decode.tiles_image,
        projection_image: b.projection_image,
        guarded_image: b.guarded_image,
    }
}
pub(crate) fn request(b: &Bootstrap, position: u32, previous: u32) -> Request {
    Request {
        protocol: 1,
        id: u64::from(position) + 1,
        device_ids: b.device_ids,
        session: b.scope.session,
        registration: b.registration,
        profile_sha256: b.sha256().unwrap(),
        command: Command::Forward {
            generation: u64::from(position) + 1,
            token: b
                .prompt_tokens
                .get(position as usize)
                .copied()
                .unwrap_or(previous),
            cache_metadata: std::iter::once(position).chain(0..144).collect(),
            rotary_bits: vec![0; 128],
        },
    }
}
pub(crate) fn close_request(b: &Bootstrap) -> Request {
    let mut value = request(b, 0, 0);
    value.id = u64::from(b.profile.forwards()) + 1;
    value.command = Command::Close;
    value
}
pub(crate) fn control(generation: u64) -> Control {
    four::tests::control(generation)
}
pub(crate) fn payload(output: u32) -> Vec<u8> {
    let mut bytes = vec![0; OBSERVATION_BYTES];
    let offset = OBSERVATION_BYTES - LOGIT_BYTES + output as usize * 2;
    bytes[offset..offset + 2].copy_from_slice(&0x3f80u16.to_le_bytes());
    bytes
}
fn frame(b: &Bootstrap, position: u32) -> (Frame, Control, Vec<u8>) {
    let request = request(b, position, 7);
    let control = control(u64::from(position) + 1);
    let bytes = payload(7);
    let completion =
        Completion::from_observation(b.profile, &request, &control, &bytes, 7).unwrap();
    (
        Frame {
            schema: RESPONSE_SCHEMA.into(),
            profile: b.profile,
            request,
            completion,
        },
        control,
        bytes,
    )
}
// Synthetic transcript records are not claims that their uncaptured bodies exist.
fn symbolic_frame(b: &Bootstrap, position: u32, previous: u32) -> Frame {
    let request = request(b, position, previous);
    let completion = Completion {
        generation: u64::from(position) + 1,
        position,
        input_token: if position < 2048 {
            b.prompt_tokens[position as usize]
        } else {
            previous
        },
        output_token: position % 101 + 3,
        bank: BankStep::at(b.profile, position).unwrap(),
        control: Part {
            bytes: CONTROL_BYTES as u32,
            sha256: [1; 32],
        },
        observation: Part {
            bytes: OBSERVATION_BYTES as u32,
            sha256: [2; 32],
        },
        logits: Part {
            bytes: LOGIT_BYTES as u32,
            sha256: [3; 32],
        },
        captured: b.profile.captures(position),
        first_frontiers: [(u64::from(position) * 1000 + 1, u64::from(position) * 1000); 2],
        final_frontiers: [(
            u64::from(position) * 1000 + 361,
            u64::from(position) * 1000 + 359,
        ); 2],
        chain: [0; 32],
    };
    Frame {
        schema: RESPONSE_SCHEMA.into(),
        profile: b.profile,
        request,
        completion,
    }
}

#[test]
fn long_profiles_and_close_ids_are_domain_separated_and_ar4_stays_closed() {
    let full = bootstrap(Profile::Full2303);
    let ready = bootstrap(Profile::Readiness40);
    assert_ne!(full.sha256().unwrap(), ready.sha256().unwrap());
    assert_eq!(full.profile.outputs(), 256);
    assert_eq!(ready.profile.outputs(), 0);
    assert_eq!(ready.profile.capture_positions(), [0, 15, 16, 39]);
    close_request(&full).validate(Profile::Full2303).unwrap();
    close_request(&ready)
        .validate(Profile::Readiness40)
        .unwrap();
    assert!(close_request(&full).validate(Profile::Readiness40).is_err());
    assert!(close_request(&ready).validate(Profile::Full2303).is_err());
    assert!(
        request(&ready, 40, 0)
            .validate(Profile::Readiness40)
            .is_err()
    );
    assert!(request(&full, 2303, 0).validate(Profile::Full2303).is_err());
    assert_eq!(four::FORWARDS, 4);
    assert!(control(5).validate(5).is_err());
    assert!(serde_json::from_str::<Profile>("\"full40\"").is_err());
}

#[test]
fn long_bootstrap_identity_images_prompt_and_combined_budget_are_closed() {
    let b = bootstrap(Profile::Full2303);
    b.validate(b.device_ids, b.timeout_ms, b.scope.child_identity)
        .unwrap();
    for edit in [
        |b: &mut Bootstrap| b.prompt_tokens.pop().map(|_| ()).unwrap(),
        |b: &mut Bootstrap| b.prompt_tokens[0] = VOCABULARY,
        |b: &mut Bootstrap| b.registration[0] ^= 1,
        |b: &mut Bootstrap| b.begin.scope.session[0] ^= 1,
        |b: &mut Bootstrap| b.guarded_image.sha256[0] ^= 1,
        |b: &mut Bootstrap| b.begin.tail_image = None,
        |b: &mut Bootstrap| b.prefix_image.bytes = 0,
        |b: &mut Bootstrap| b.mlp_image.bytes = (32 << 20) + 1,
        |b: &mut Bootstrap| {
            b.mlp_image.bytes = 32 << 20;
            b.prefix_image.bytes = 32 << 20;
        },
    ] {
        let mut bad = b.clone();
        edit(&mut bad);
        assert!(bad.sha256().is_err());
    }
    assert!(
        b.validate([1, 1], b.timeout_ms, b.scope.child_identity)
            .is_err()
    );
    assert!(
        b.validate(b.device_ids, b.timeout_ms + 1, b.scope.child_identity)
            .is_err()
    );
    assert!(b.validate(b.device_ids, b.timeout_ms, 0).is_err());
}

#[test]
fn long_bank_generations_and_page_edges_are_exact() {
    for position in 0..FORWARDS {
        let step = BankStep::at(Profile::Full2303, position).unwrap();
        assert_eq!(step.bank, position % 2);
        assert_eq!(step.local_generation, position / 2 + 1);
        assert_eq!(step.logical_page * 16 + step.page_offset, position);
        assert_eq!(
            step.retired_forward,
            (position >= 2).then(|| u64::from(position) - 1)
        );
    }
    assert_eq!(
        BankStep::at(Profile::Full2303, 2302).unwrap(),
        BankStep {
            bank: 0,
            local_generation: 1152,
            retired_forward: Some(2301),
            logical_page: 143,
            page_offset: 14,
        }
    );
    for position in [15, 16, 2047, 2048] {
        assert_eq!(
            BankStep::at(Profile::Full2303, position)
                .unwrap()
                .page_offset,
            position % 16
        );
    }
    assert!(BankStep::at(Profile::Readiness40, 40).is_err());
}

#[test]
fn long_control_decoder_preserves_ar4_predicates_and_long_generation() {
    for generation in [1, 2, 3, 4, 40, 2048, 2049, 2303] {
        let c = control(generation);
        let raw = c.encode();
        assert_eq!(
            decode_control(&raw, Profile::Full2303, generation).unwrap(),
            c
        );
        if generation <= 4 {
            assert_eq!(four::Control::decode(&raw, generation).unwrap(), c);
        } else {
            assert!(four::Control::decode(&raw, generation).is_err());
        }
    }
    for edit in [
        |c: &mut Control| c.layers[0].prefix_states[0][0] = 0,
        |c: &mut Control| c.layers[35].prefix_states[1][283] = 63,
        |c: &mut Control| c.layers[1].mlp_prefixes[1][0] = 0,
        |c: &mut Control| c.layers[4].guards[1][0] += 1,
        |c: &mut Control| c.layers[5].guards[0][1] = 1,
        |c: &mut Control| c.layers[6].observed_queue_frontiers[1] = (1, 2),
        |c: &mut Control| c.layers[7].observed_queue_frontiers[0] = (1, 0),
    ] {
        let mut bad = control(2303);
        edit(&mut bad);
        assert!(decode_control(&bad.encode(), Profile::Full2303, 2303).is_err());
    }
    let raw = control(1).encode();
    assert!(decode_control(&raw[..raw.len() - 1], Profile::Full2303, 1).is_err());
    assert!(decode_control(&[raw.as_slice(), &[0]].concat(), Profile::Full2303, 1).is_err());
    assert!(decode_control(&raw, Profile::Full2303, 0).is_err());
    assert!(decode_control(&control(41).encode(), Profile::Readiness40, 41).is_err());
}

#[test]
fn long_maximal_selected_2302_header_and_total_retention_fit_existing_caps() {
    let b = bootstrap(Profile::Full2303);
    let (mut f, _, _) = frame(&b, 2302);
    f.request.device_ids = [u64::MAX, u64::MAX - 1];
    f.request.session = [255; 32];
    f.request.registration = [255; 32];
    f.request.profile_sha256 = [255; 32];
    if let Command::Forward {
        token, rotary_bits, ..
    } = &mut f.request.command
    {
        *token = VOCABULARY - 1;
        rotary_bits.fill(0xff7fffff);
    }
    f.completion.input_token = VOCABULARY - 1;
    f.completion.output_token = VOCABULARY - 1;
    f.completion.control.sha256 = [255; 32];
    f.completion.observation.sha256 = [255; 32];
    f.completion.logits.sha256 = [255; 32];
    f.completion.chain = [255; 32];
    f.completion.first_frontiers = [(u64::MAX - 1, u64::MAX - 2); 2];
    f.completion.final_frontiers = [(u64::MAX, u64::MAX - 1); 2];
    f.validate(Profile::Full2303).unwrap();
    assert!(header_bytes(&f).unwrap().len() < RECORD_BYTES);
    assert_eq!(CAPTURE_BYTES, 3_399_200);
    assert!(MAX_RETAINED_BYTES < EVIDENCE_BYTES);
    assert!(MAX_RETAINED_BYTES < STREAM_BYTES);
    assert!(OUTPUT_TOKENS * LOGIT_BYTES > STREAM_BYTES);
    let mut budget = FrameBudget::new();
    budget.charge(STREAM_BYTES).unwrap();
    assert!(budget.charge(1).is_err());
}

#[test]
fn long_selected_and_unselected_frames_round_trip_with_derived_part_pins() {
    for profile in [Profile::Full2303, Profile::Readiness40] {
        let b = bootstrap(profile);
        for position in profile.capture_positions().into_iter().chain([1]) {
            let (f, c, bytes) = frame(&b, position);
            let mut encoded = Vec::new();
            let mut sent = FrameBudget::new();
            write_frame(&mut encoded, &mut sent, &f, &c, &bytes).unwrap();
            let mut received = FrameBudget::new();
            let mut input = encoded.as_slice();
            let (actual, capture) = read_frame(&mut input, &mut received, profile)
                .unwrap()
                .unwrap();
            assert_eq!(actual, f);
            assert!(input.is_empty());
            assert_eq!(sent.used(), received.used());
            if profile.captures(position) {
                let capture = capture.unwrap();
                assert_eq!(capture.control, c);
                assert_eq!(capture.observation, bytes);
                assert_eq!(capture.parts, Payload::from_bytes(&bytes).unwrap());
            } else {
                assert!(capture.is_none());
                assert!(encoded.len() <= RECORD_BYTES + 4);
            }
        }
    }
}

#[test]
fn long_selected_control_rehash_cannot_hide_invalid_terminal_or_frontiers() {
    let b = bootstrap(Profile::Full2303);
    for corrupt_frontier in [false, true] {
        let (mut f, mut c, bytes) = frame(&b, 2302);
        if corrupt_frontier {
            f.completion.final_frontiers[0].0 += 1;
        } else {
            c.layers[35].guards[1][2] = 0;
        }
        let raw = c.encode();
        f.completion.control = part(&raw);
        let mut encoded = Vec::new();
        write_record(&mut encoded, &mut FrameBudget::new(), &f).unwrap();
        encoded.extend_from_slice(&raw);
        encoded.extend_from_slice(&bytes);
        assert!(read_frame(&mut encoded.as_slice(), &mut FrameBudget::new(), b.profile).is_err());
    }
}

#[test]
fn long_nonfinite_payload_wrong_argmax_and_selected_byte_mutation_are_refused() {
    let b = bootstrap(Profile::Full2303);
    let (f, c, bytes) = frame(&b, 0);
    assert!(Completion::from_observation(b.profile, &f.request, &c, &bytes, 8).is_err());
    for offset in [0, 36 * 8192, OBSERVATION_BYTES - 2] {
        let mut bad = bytes.clone();
        bad[offset..offset + 2].copy_from_slice(&0x7fc0u16.to_le_bytes());
        assert!(Completion::from_observation(b.profile, &f.request, &c, &bad, 7).is_err());
    }
    let mut encoded = Vec::new();
    write_frame(&mut encoded, &mut FrameBudget::new(), &f, &c, &bytes).unwrap();
    *encoded.last_mut().unwrap() ^= 1;
    assert!(read_frame(&mut encoded.as_slice(), &mut FrameBudget::new(), b.profile).is_err());
    let zeros = vec![0; OBSERVATION_BYTES];
    assert_eq!(argmax(&zeros).unwrap(), 0);
    let mut tied = payload(7);
    tied[OBSERVATION_BYTES - LOGIT_BYTES..OBSERVATION_BYTES - LOGIT_BYTES + 2]
        .copy_from_slice(&0x3f80u16.to_le_bytes());
    assert_eq!(argmax(&tied).unwrap(), 0);
}

#[test]
fn long_record_refuses_oversize_truncation_unknown_fields_and_trailing_bytes() {
    let b = bootstrap(Profile::Readiness40);
    let r = request(&b, 0, 0);
    let mut encoded = Vec::new();
    write_record(&mut encoded, &mut FrameBudget::new(), &r).unwrap();
    for n in [1, 3, encoded.len() - 1] {
        assert!(read_record::<Request>(&mut &encoded[..n], &mut FrameBudget::new()).is_err());
    }
    let too_large = (RECORD_BYTES as u32 + 1).to_le_bytes();
    assert!(read_record::<Request>(&mut too_large.as_slice(), &mut FrameBudget::new()).is_err());
    for suffix in [b"x".as_slice(), b" {}".as_slice()] {
        let raw = [header_bytes(&r).unwrap().as_slice(), suffix].concat();
        let encoded = [(raw.len() as u32).to_le_bytes().as_slice(), &raw].concat();
        assert!(read_record::<Request>(&mut encoded.as_slice(), &mut FrameBudget::new()).is_err());
    }
    let mut value = serde_json::to_value(&r).unwrap();
    value["retry"] = true.into();
    assert!(serde_json::from_value::<Request>(value).is_err());
    let (f, c, bytes) = frame(&b, 0);
    let mut encoded = Vec::new();
    write_frame(&mut encoded, &mut FrameBudget::new(), &f, &c, &bytes).unwrap();
    encoded.push(b'x');
    let mut input = encoded.as_slice();
    let mut budget = FrameBudget::new();
    read_frame(&mut input, &mut budget, b.profile)
        .unwrap()
        .unwrap();
    assert!(read_frame(&mut input, &mut budget, b.profile).is_err());
}

#[test]
fn long_transcript_refuses_prompt_pages_scope_pending_and_chain_mutations() {
    let b = bootstrap(Profile::Full2303);
    let r = request(&b, 0, 0);
    for edit in [
        |r: &mut Request| r.device_ids.swap(0, 1),
        |r: &mut Request| r.session[0] ^= 1,
        |r: &mut Request| r.profile_sha256[0] ^= 1,
        |r: &mut Request| {
            if let Command::Forward { token, .. } = &mut r.command {
                *token += 1;
            }
        },
        |r: &mut Request| {
            if let Command::Forward { cache_metadata, .. } = &mut r.command {
                cache_metadata[2] = cache_metadata[1];
            }
        },
        |r: &mut Request| {
            if let Command::Forward { rotary_bits, .. } = &mut r.command {
                rotary_bits[0] = f32::INFINITY.to_bits();
            }
        },
    ] {
        let mut t = Transcript::new(b.clone()).unwrap();
        let mut bad = r.clone();
        edit(&mut bad);
        assert!(t.begin(&bad).is_err());
        assert!(t.begin(&r).is_err());
        assert_eq!(t.completed(), 0);
    }
    let (mut f, _, _) = frame(&b, 0);
    let mut t = Transcript::new(b.clone()).unwrap();
    assert!(t.next_chain(&f.request, &f.completion).is_err());
    t.begin(&f.request).unwrap();
    f.completion.chain = t.next_chain(&f.request, &f.completion).unwrap();
    t.advance(&f).unwrap();
    let mut changed = request(&b, 1, 7);
    if let Command::Forward { cache_metadata, .. } = &mut changed.command {
        cache_metadata.swap(1, 2);
    }
    assert!(t.begin(&changed).is_err());
    assert_eq!(t.completed(), 1);
    let mut t = Transcript::new(b.clone()).unwrap();
    t.begin(&r).unwrap();
    f.completion.chain[0] ^= 1;
    assert!(t.advance(&f).is_err());
    assert_eq!(t.completed(), 0);
    let mut t = Transcript::new(b).unwrap();
    t.begin(&r).unwrap();
    assert!(t.begin(&r).is_err());
    assert!(t.advance(&f).is_err());
}

#[test]
fn long_transcript_full_2303_commits_only_own_outputs_and_exact_close() {
    let b = bootstrap(Profile::Full2303);
    let mut t = Transcript::new(b.clone()).unwrap();
    let mut previous = 0;
    for position in 0..FORWARDS {
        let mut f = symbolic_frame(&b, position, previous);
        t.begin(&f.request).unwrap();
        f.completion.chain = t.next_chain(&f.request, &f.completion).unwrap();
        t.advance(&f).unwrap();
        previous = f.completion.output_token;
    }
    assert_eq!(t.completed(), 2303);
    assert_eq!(t.output_tokens().len(), 256);
    assert_eq!(t.output_tokens()[0], 2047 % 101 + 3);
    let digest = t.digest();
    t.close(&close_request(&b), digest).unwrap();
    assert!(t.is_closed());
    assert!(t.close(&close_request(&b), digest).is_err());
    assert!(!t.is_closed());
}

#[test]
fn long_generated_input_must_be_the_committed_predecessor_not_a_reference_token() {
    let b = bootstrap(Profile::Full2303);
    let mut t = Transcript::new(b.clone()).unwrap();
    let mut previous = 0;
    for position in 0..2048 {
        let mut f = symbolic_frame(&b, position, previous);
        t.begin(&f.request).unwrap();
        f.completion.chain = t.next_chain(&f.request, &f.completion).unwrap();
        t.advance(&f).unwrap();
        previous = f.completion.output_token;
    }
    assert_eq!(t.output_tokens(), &[previous]);
    let wrong = request(&b, 2048, previous + 1);
    assert!(t.begin(&wrong).is_err());
    assert_eq!(t.completed(), 2048);
    assert!(t.begin(&request(&b, 2048, previous)).is_err());
}
