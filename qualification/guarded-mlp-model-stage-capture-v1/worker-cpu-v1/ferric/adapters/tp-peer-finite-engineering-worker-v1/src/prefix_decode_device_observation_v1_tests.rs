//! Synthetic CPU fixtures only. No native timestamp or execution evidence.
use super::*;
use crate::finite_prefix_decode_wire_v1::{
    self as wire,
    tests::{bootstrap, completed, request},
};

pub(crate) fn report(mode: wire::InputMode) -> Report {
    let b = bootstrap(mode);
    let images = Images {
        prefix: b.prefix_image.sha256,
        mlp: b.tiles_image.sha256,
        residual: b.begin.residual_image.sha256,
        tail: b.begin.tail_image.unwrap().sha256,
        copy: b.begin.residual_image.sha256,
    };
    let ranks = core::array::from_fn(|rank| Rank {
        rank: rank as u32,
        unique_id: b.device_ids[rank],
        queue_epoch: 11 + rank as u64,
    });
    let mut packets = [0; 2];
    let control = wire::tests::control();
    let host_times: Vec<_> = control
        .embedding_ns
        .iter()
        .copied()
        .chain(
            control
                .layers
                .iter()
                .flat_map(|l| l.paired_ns.iter().flatten().copied()),
        )
        .chain(control.tail_ns.iter().copied())
        .collect();
    let rows = (0..MAX_ROWS)
        .map(|i| {
            let (generation, position, stage, layer, rank) = expected(i).unwrap();
            let packet_id = packets[rank as usize];
            packets[rank as usize] += 1;
            Row {
                generation,
                position,
                stage,
                layer,
                rank,
                entry: stage.entry().into(),
                image_sha256: images.image(stage),
                group_incarnation: 7,
                unique_id: ranks[rank as usize].unique_id,
                queue_epoch: ranks[rank as usize].queue_epoch,
                packet_id,
                signal_generation: packet_id + 1,
                start_tick: 100 + i as u64,
                end_tick: 100 + i as u64,
                host_elapsed_ns: host_times[i % PER_FORWARD],
            }
        })
        .collect();
    let mut chain = wire::Chain::new(b.registration, b.sha256().unwrap());
    let mut previous = None;
    let completions = (0..4)
        .map(|position| {
            let (response, _, _) =
                completed(&request(&b, position, previous), &mut chain, 100 + position);
            let wire::Event::Completed(c) = response.event else {
                panic!("synthetic completion");
            };
            previous = Some(c.output_token);
            c
        })
        .collect();
    Report {
        schema: SCHEMA.into(),
        profile_sha256: b.sha256().unwrap(),
        child_pid: b.scope.child_identity,
        bootstrap: b,
        worker_sha256: [9; 32],
        group_incarnation: 7,
        ranks,
        images,
        rows,
        final_dispatches: packets,
        completions,
        transcript_sha256: chain.digest(),
        raw_timestamp_queue: true,
        shared_full_currentness: true,
        cache_kernel_admission: false,
        operational_currentness: false,
        native_closed: true,
        raw_completion_ticks: true,
        calibrated_nanoseconds: false,
        cross_device_clock_alignment: false,
        overlap_claim: false,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
        full_model_acceptance: false,
    }
}
#[test]
fn device_raw_schema_roundtrips_tf_and_ar_with_exact_bounded_census() {
    for mode in [
        wire::InputMode::TeacherForced,
        wire::InputMode::Autoregressive,
    ] {
        let value = report(mode);
        let bytes = value.encode().unwrap();
        assert!(bytes.len() < MAX_BYTES);
        let decoded = Report::decode(&bytes).unwrap();
        assert_eq!(decoded.rows, value.rows);
        assert_eq!(decoded.completions, value.completions);
        assert_eq!(decoded.final_dispatches, [592, 580]);
        assert!(crate::prefix_decode_host_observation_v1::Report::decode(&bytes).is_err());
        assert!(crate::prefix_decode_host_observation_v2::Report::decode(&bytes).is_err());
    }
}
#[test]
fn device_raw_inventory_has_exact_stages_layers_and_rank_counts() {
    let value = report(wire::InputMode::TeacherForced);
    for forward in value.rows.chunks_exact(PER_FORWARD) {
        assert_eq!(forward.iter().filter(|r| r.rank == 0).count(), 148);
        assert_eq!(forward.iter().filter(|r| r.rank == 1).count(), 145);
        assert_eq!(forward[0].stage, Stage::Embedding);
        assert_eq!(forward[1].stage, Stage::Copy);
        for layer in 0..36 {
            for (slot, stage) in [
                Stage::Prefix,
                Stage::PostAttentionResidual,
                Stage::Mlp,
                Stage::PostMlpResidual,
            ]
            .into_iter()
            .enumerate()
            {
                for rank in 0..2 {
                    let row = &forward[2 + layer * 8 + slot * 2 + rank];
                    assert_eq!(
                        (row.stage, row.layer, row.rank),
                        (stage, Some(layer as u32), rank as u32)
                    );
                }
            }
        }
        assert_eq!(
            forward[290..].iter().map(|r| r.stage).collect::<Vec<_>>(),
            [Stage::FinalNorm, Stage::Head, Stage::Argmax]
        );
    }
    assert!(expected(MAX_ROWS).is_err());
    assert!(expected(usize::MAX).is_err());
}
#[test]
fn device_raw_rejects_each_row_identity_and_label_substitution() {
    for index in [0, 1, 145, 290, 292, 293, MAX_ROWS - 1] {
        for change in 0..12 {
            let mut value = report(wire::InputMode::TeacherForced);
            let r = &mut value.rows[index];
            match change {
                0 => r.generation += 1,
                1 => r.position += 1,
                2 => r.stage = Stage::Head,
                3 => r.layer = Some(36),
                4 => r.rank ^= 1,
                5 => r.entry.push('x'),
                6 => r.image_sha256[0] ^= 1,
                7 => r.group_incarnation += 1,
                8 => r.unique_id ^= 1,
                9 => r.queue_epoch += 1,
                10 => r.packet_id += 1,
                _ => r.signal_generation += 1,
            }
            assert!(value.validate().is_err(), "{index}/{change}");
        }
    }
}
#[test]
fn device_raw_rejects_zero_reversed_ticks_but_accepts_equal_nonzero() {
    report(wire::InputMode::TeacherForced).validate().unwrap();
    for ticks in [[0, 0], [0, 9], [9, 0], [9, 8]] {
        let mut value = report(wire::InputMode::TeacherForced);
        value.rows[0].start_tick = ticks[0];
        value.rows[0].end_tick = ticks[1];
        assert!(value.validate().is_err());
    }
}
#[test]
fn device_raw_rejects_missing_duplicate_extra_or_reordered_rows() {
    for change in 0..4 {
        let mut value = report(wire::InputMode::TeacherForced);
        match change {
            0 => {
                value.rows.pop();
            }
            1 => value.rows[500] = value.rows[499].clone(),
            2 => value.rows.push(value.rows[0].clone()),
            _ => value.rows.swap(2, 3),
        }
        assert!(value.validate().is_err());
    }
}
#[test]
fn device_raw_rejects_queue_rollover_and_unrecorded_setup_packet() {
    for offset in [1, u64::MAX - 1000] {
        let mut value = report(wire::InputMode::TeacherForced);
        for row in &mut value.rows {
            row.packet_id += offset;
            row.signal_generation += offset;
        }
        assert!(value.validate().is_err());
    }
    let mut value = report(wire::InputMode::TeacherForced);
    for row in &mut value.rows[293..] {
        row.queue_epoch += 1;
    }
    assert!(value.validate().is_err());
}
#[test]
fn device_raw_rejects_all_policy_and_authority_mutations() {
    for change in 0..12 {
        let mut value = report(wire::InputMode::TeacherForced);
        match change {
            0 => value.raw_timestamp_queue = false,
            1 => value.shared_full_currentness = false,
            2 => value.cache_kernel_admission = true,
            3 => value.operational_currentness = true,
            4 => value.native_closed = false,
            5 => value.raw_completion_ticks = false,
            6 => value.calibrated_nanoseconds = true,
            7 => value.cross_device_clock_alignment = true,
            8 => value.overlap_claim = true,
            9 => value.numerical_acceptance = true,
            10 => value.performance_claim = true,
            _ => value.production_authority = true,
        }
        assert!(value.validate().is_err());
    }
    let mut value = report(wire::InputMode::TeacherForced);
    value.full_model_acceptance = true;
    assert!(value.validate().is_err());
}
#[test]
fn device_raw_rejects_process_profile_image_group_or_final_count_drift() {
    for change in 0..10 {
        let mut value = report(wire::InputMode::TeacherForced);
        match change {
            0 => value.schema.push('x'),
            1 => value.child_pid += 1,
            2 => value.worker_sha256 = [0; 32],
            3 => value.profile_sha256[0] ^= 1,
            4 => value.group_incarnation = 0,
            5 => value.ranks[0].rank = 1,
            6 => value.ranks[1].unique_id = value.ranks[0].unique_id,
            7 => value.images.prefix[0] ^= 1,
            8 => value.images.tail = [0; 32],
            _ => value.final_dispatches[0] -= 1,
        }
        assert!(value.validate().is_err());
    }
    // Internally consistent replacement objects must still match the input images.
    for stage in [Stage::Embedding, Stage::Copy] {
        let mut value = report(wire::InputMode::TeacherForced);
        if stage == Stage::Copy {
            value.images.copy[0] ^= 1;
        } else {
            value.images.tail[0] ^= 1;
        }
        for row in &mut value.rows {
            row.image_sha256 = value.images.image(row.stage);
        }
        assert!(value.validate().is_err());
    }
}
#[test]
fn device_raw_rejects_completion_history_capture_and_chain_drift() {
    for change in 0..8 {
        let mut value = report(wire::InputMode::Autoregressive);
        match change {
            0 => {
                value.completions.pop();
            }
            1 => value.completions[1].input_token ^= 1,
            2 => value.completions[0].position += 1,
            3 => value.completions[2].control.bytes -= 1,
            4 => value.completions[2].capture.total.sha256[0] ^= 1,
            5 => value.completions[3].chain[0] ^= 1,
            6 => value.transcript_sha256[0] ^= 1,
            _ => value.completions.push(value.completions[0].clone()),
        }
        assert!(value.validate().is_err());
    }
}
#[test]
fn device_raw_decoder_refuses_unknown_duplicate_legacy_and_oversize_input() {
    let bytes = report(wire::InputMode::TeacherForced).encode().unwrap();
    for prefix in [
        b"{\"extra\":0,".as_slice(),
        b"{\"schema\":\"duplicate\",".as_slice(),
    ] {
        let mut changed = prefix.to_vec();
        changed.extend_from_slice(&bytes[1..]);
        assert!(Report::decode(&changed).is_err());
    }
    let legacy =
        crate::prefix_decode_host_observation_v2::tests::report(wire::InputMode::TeacherForced);
    assert!(Report::decode(&serde_json::to_vec(&legacy).unwrap()).is_err());
    assert!(Report::decode(&vec![b' '; MAX_BYTES + 1]).is_err());
    assert!(Report::decode(&[]).is_err());
}
#[test]
fn device_raw_writer_refuses_overflow_before_retaining_more_bytes() {
    use std::io::Write;
    let mut writer = Bounded(vec![0; MAX_BYTES]);
    assert!(writer.write_all(&[1]).is_err());
    assert_eq!(writer.0.len(), MAX_BYTES);
}

#[test]
fn device_raw_control_join_checks_all_intervals_digest_and_states() {
    let value = report(wire::InputMode::TeacherForced);
    let control = wire::tests::control();
    for position in 0..4 {
        value.validate_control(position, &control).unwrap();
    }
    assert!(value.validate_control(4, &control).is_err());
    for index in 0..PER_FORWARD {
        let mut changed = value.clone();
        changed.rows[index].host_elapsed_ns ^= 1;
        assert!(
            changed.validate_control(0, &control).is_err(),
            "interval {index}"
        );
    }
    let mut changed = control.clone();
    changed.layers[0].prefix_states[0][0] ^= 1;
    assert!(value.validate_control(0, &changed).is_err());
    let mut changed = value;
    changed.completions[0].control.sha256[0] ^= 1;
    assert!(changed.validate_control(0, &control).is_err());
}
