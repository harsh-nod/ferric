//! Explicit synthetic CPU fixtures, not evidence of native clock sampling.
use super::*;
use crate::finite_prefix_decode_wire_v1::InputMode;

pub(crate) fn report() -> Report {
    let raw = raw::tests::report(InputMode::TeacherForced);
    let samples = (0..SAMPLE_COUNT)
        .map(|i| {
            let rank = i % 2;
            Sample {
                generation: (i / 4) as u64 + 1,
                position: (i / 4) as u32,
                endpoint: if i % 4 < 2 {
                    Endpoint::Pre
                } else {
                    Endpoint::Post
                },
                rank: rank as u32,
                row_boundary: ((i / 4 + usize::from(i % 4 >= 2)) * raw::PER_FORWARD) as u32,
                group_incarnation: raw.group_incarnation,
                unique_id: raw.ranks[rank].unique_id,
                queue_epoch: raw.ranks[rank].queue_epoch,
                gpu_id: 10 + rank as u32,
                gpu_clock_counter: 1000 + i as u64,
                cpu_clock_counter: 2000 + i as u64,
                system_clock_counter: 3000 + i as u64,
                system_clock_frequency_hz: 1_000_000_000,
                host_started_ns: i as u64 * 10,
                host_finished_ns: i as u64 * 10 + 5,
            }
        })
        .collect();
    Report {
        schema: SCHEMA.into(),
        raw,
        samples,
        raw_clock_counters: true,
        clock_domain_validated: false,
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
fn clock_schema_roundtrip_preserves_raw_report_and_sixteen_samples() {
    let value = report();
    let bytes = value.encode().unwrap();
    let decoded = Report::decode(&bytes).unwrap();
    assert_eq!(decoded.samples, value.samples);
    assert_eq!(decoded.raw.rows, value.raw.rows);
    assert_eq!(decoded.raw.completions, value.raw.completions);
    assert_eq!(decoded.raw.rows.len(), 1172);
    assert!(bytes.len() < MAX_BYTES);
    assert!(raw::Report::decode(&bytes).is_err());
    assert!(Report::decode(&value.raw.encode().unwrap()).is_err());
}
#[test]
fn clock_schema_refuses_missing_extra_duplicate_and_reordered_samples() {
    for kind in 0..4 {
        let mut value = report();
        match kind {
            0 => {
                value.samples.pop();
            }
            1 => value.samples.push(value.samples[15].clone()),
            2 => value.samples[9] = value.samples[8].clone(),
            _ => value.samples.swap(6, 7),
        }
        assert!(value.validate().is_err());
    }
}
#[test]
fn clock_schema_refuses_endpoint_boundary_and_native_identity_drift() {
    for index in [0, 7, 15] {
        for field in 0..9 {
            let mut value = report();
            let sample = &mut value.samples[index];
            match field {
                0 => sample.generation ^= 1,
                1 => sample.position ^= 1,
                2 => {
                    sample.endpoint = if sample.endpoint == Endpoint::Pre {
                        Endpoint::Post
                    } else {
                        Endpoint::Pre
                    }
                }
                3 => sample.rank ^= 1,
                4 => sample.row_boundary ^= 1,
                5 => sample.group_incarnation ^= 1,
                6 => sample.unique_id ^= 1,
                7 => sample.queue_epoch ^= 1,
                _ => sample.system_clock_frequency_hz = 0,
            }
            assert!(value.validate().is_err(), "sample {index} field {field}");
        }
    }
}
#[test]
fn clock_schema_refuses_kfd_identity_and_frequency_changes() {
    let mut value = report();
    value.samples[1].gpu_id = value.samples[0].gpu_id;
    assert!(value.validate().is_err());
    for field in 0..2 {
        let mut value = report();
        if field == 0 {
            value.samples[12].gpu_id ^= 1;
        } else {
            value.samples[13].system_clock_frequency_hz += 1;
        }
        assert!(value.validate().is_err());
    }
}
#[test]
fn clock_schema_checks_host_brackets_but_does_not_infer_counter_domains() {
    let mut value = report();
    value.samples[2].host_finished_ns = value.samples[2].host_started_ns - 1;
    assert!(value.validate().is_err());
    let mut value = report();
    value.samples[5].host_started_ns = value.samples[4].host_finished_ns - 1;
    assert!(value.validate().is_err());
    let mut value = report();
    for sample in &mut value.samples {
        sample.gpu_clock_counter = 0;
        sample.cpu_clock_counter = u64::MAX - sample.generation;
        sample.system_clock_counter = 0;
    }
    value.validate().unwrap();
    assert!(!value.clock_domain_validated && !value.calibrated_nanoseconds);
}
#[test]
fn clock_schema_refuses_authority_flags_and_invalid_inner_raw_report() {
    for flag in 0..10 {
        let mut value = report();
        match flag {
            0 => value.raw_clock_counters = false,
            1 => value.clock_domain_validated = true,
            2 => value.calibrated_nanoseconds = true,
            3 => value.cross_device_clock_alignment = true,
            4 => value.overlap_claim = true,
            5 => value.numerical_acceptance = true,
            6 => value.performance_claim = true,
            7 => value.production_authority = true,
            8 => value.full_model_acceptance = true,
            _ => value.raw.native_closed = false,
        }
        assert!(value.validate().is_err());
    }
}
#[test]
fn clock_schema_retains_exact_control_join_and_bounds() {
    use std::io::Write;
    let value = report();
    let control = crate::finite_prefix_decode_wire_v1::tests::control();
    value.validate_control(3, &control).unwrap();
    let mut bad = control;
    bad.tail_ns[1] ^= 1;
    assert!(value.validate_control(3, &bad).is_err());
    assert!(value.validate_control(4, &bad).is_err());
    assert!(Report::decode(&[]).is_err());
    assert!(Report::decode(&vec![b' '; MAX_BYTES + 1]).is_err());
    let mut writer = Bounded(vec![0; MAX_BYTES - 1]);
    assert!(writer.write_all(&[1, 2]).is_err());
    assert_eq!(writer.0.len(), MAX_BYTES - 1);
    let mut object = serde_json::to_value(value).unwrap();
    object
        .as_object_mut()
        .unwrap()
        .insert("unknown".into(), true.into());
    assert!(Report::decode(&serde_json::to_vec(&object).unwrap()).is_err());
}
