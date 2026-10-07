#[cfg(not(feature = "model-timestamps"))]
mod ordered64_tests {
    use super::*;

    fn worker(mode: &str) -> Worker {
        let mut worker = Worker::connect(fake(mode), 1, Duration::from_secs(5)).unwrap();
        worker.options.ordered_batches = true;
        worker.options.ordered64 = true;
        worker
    }

    fn header(count: usize) -> CommandV1 {
        let CommandV1::DispatchOrderedBatch {
            dispatches,
            timeout_ms,
        } = ordered_header(count)
        else {
            unreachable!()
        };
        CommandV1::DispatchOrderedBatch64 {
            dispatches,
            timeout_ms,
        }
    }

    #[test]
    fn ordered64_bounds_short_tail_and_packet_retirement_are_exact() {
        let mut worker = worker("normal");
        for count in [16, 17, 64, 1] {
            worker.send(header(count), vec![]).unwrap();
            assert!(
                matches!(worker.pending, Some(PendingRequest::OrderedBatch64(n)) if n == count)
            );
            worker.wait_ordered_batch(count).unwrap();
        }
        assert_eq!(worker.queue_packets, 98);
        for count in [0, 65] {
            assert!(worker.send(header(count), vec![]).is_err());
            assert!(worker.pending.is_none());
            assert_eq!(worker.queue_packets, 98);
        }
        worker.send(header(17), vec![]).unwrap();
        worker.close().unwrap();
        assert_eq!(worker.queue_packets, 115);
        assert!(worker.exited);
    }

    #[test]
    fn ordered64_wrong_kind_count_payload_error_and_timeout_are_terminal() {
        for mode in [
            "orderedwrongkind",
            "orderedshort",
            "orderedsequence",
            "orderedpayload",
            "orderederror",
            "orderedstall",
        ] {
            let mut worker = worker(mode);
            if mode == "orderedstall" {
                worker.timeout = Duration::from_millis(100);
            }
            worker.send(header(17), vec![]).unwrap();
            assert!(worker.wait_ordered_batch(17).is_err(), "{mode}");
            assert!(worker.failed && worker.exited);
            assert_eq!(worker.queue_packets, 0);
            assert!(worker.send(header(1), vec![]).is_err());
        }
    }

    #[test]
    fn ordered64_rejects_mismatched_wait_or_host_command_before_retirement() {
        for operation in 0..4 {
            let mut worker = worker("normal");
            let buffer = worker.allocate(4).unwrap();
            worker.send(header(64), vec![]).unwrap();
            let result = match operation {
                0 => worker.wait_ordered_batch(16),
                1 => worker.wait_sequence(64),
                2 => worker.write(buffer, 0, &[0; 4]),
                _ => worker.read(buffer, 0, &mut [0; 4]),
            };
            assert!(result.is_err());
            assert!(worker.failed && worker.exited);
            assert_eq!(worker.queue_packets, 0);
        }
    }

    #[test]
    fn ordered64_never_mixes_wire_modes_even_for_short_groups_or_after_rollover() {
        for wide in [false, true] {
            let mut worker = worker("normal");
            worker.options.ordered64 = wide;
            worker.options.rollover = true;
            let first = if wide { header(16) } else { ordered_header(16) };
            worker.send(first, vec![]).unwrap();
            worker.wait_ordered_batch(16).unwrap();
            worker.queue_packets = wire::MAX_UNRETIRED_RING_PACKETS_V1;
            worker.prepare_packets(17).unwrap();
            assert_eq!((worker.queue_epoch, worker.queue_packets), (1, 0));
            let wrong = if wide { ordered_header(1) } else { header(1) };
            assert!(worker.send(wrong, vec![]).is_err());
            assert!(worker.failed && worker.exited);
        }
    }

    #[test]
    fn ordered64_configuration_cannot_switch_after_completed_queue_use() {
        let mut worker = worker("normal");
        worker.send(header(16), vec![]).unwrap();
        worker.wait_ordered_batch(16).unwrap();
        let mut options = worker.options;
        options.ordered64 = false;
        assert!(worker.configure_options(options).is_err());
        assert!(worker.failed && worker.exited);
        assert_eq!(worker.queue_packets, 16);
    }

    #[test]
    fn ordered64_submit_rejects_65_and_unadmitted_kernel_without_publication() {
        for count in [1, 16, 17, 64, 65] {
            let mut worker = worker("normal");
            let commands = vec![
                EngineeringTpDispatchV1 {
                    kernel: "not_loaded",
                    grid_workgroups: 1,
                    workgroup_size: 64,
                    arguments: vec![],
                };
                count
            ];
            assert!(worker.submit_ordered_batch(&commands).is_err());
            assert_eq!(worker.queue_packets, 0);
            assert!(worker.failed && worker.exited);
        }
    }

    #[test]
    fn ordered64_host_diagnostic_retains_only_validated_reply_durations() {
        let timing = HostTiming::ordered64_diagnostic();
        let mut worker = worker("normal");
        worker.timing = Some(Box::new(WorkerTiming {
            timing: timing.clone(),
            rank: 0,
            pending: None,
        }));
        for count in [64, 12] {
            worker.send(header(count), vec![]).unwrap();
            worker.wait_ordered_batch(count).unwrap();
        }
        assert_eq!(worker.queue_packets, 76);
        worker.close().unwrap();
        let value = timing.snapshot();
        assert_eq!(value["incomplete"], false);
        let rows = value["records"].as_array().unwrap();
        let observed = rows
            .iter()
            .find(|row| row["category"] == "worker_reported_elapsed")
            .unwrap();
        assert_eq!(observed["count"], 2);
        assert_eq!(observed["dispatches"], 76);
        assert_eq!(observed["elapsed_ns"], 14);
        assert_eq!(observed["max_ns"], 7);
        let roundtrip = rows
            .iter()
            .find(|row| {
                row["category"] == "ipc_roundtrip" && row["label"] == "dispatch_ordered_batch64"
            })
            .unwrap();
        assert_eq!(roundtrip["count"], 2);
        assert_eq!(roundtrip["dispatches"], 76);
    }

    #[test]
    fn ordered64_legacy_host_timing_does_not_record_worker_observations() {
        let timing = HostTiming::enabled();
        let mut worker = worker("normal");
        worker.timing = Some(Box::new(WorkerTiming {
            timing: timing.clone(),
            rank: 0,
            pending: None,
        }));
        worker.send(header(64), vec![]).unwrap();
        worker.wait_ordered_batch(64).unwrap();
        worker.close().unwrap();
        let value = timing.snapshot();
        assert_eq!(value["schema"], "FerricHostTimingV1");
        assert!(
            !value["records"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["category"] == "worker_reported_elapsed")
        );
    }

    #[test]
    fn ordered64_invalid_replies_never_commit_a_worker_duration() {
        for mode in [
            "orderedwrongkind",
            "orderedshort",
            "orderedsequence",
            "orderedpayload",
            "orderederror",
        ] {
            let timing = HostTiming::ordered64_diagnostic();
            let mut worker = worker(mode);
            worker.timing = Some(Box::new(WorkerTiming {
                timing: timing.clone(),
                rank: 0,
                pending: None,
            }));
            worker.send(header(17), vec![]).unwrap();
            assert!(worker.wait_ordered_batch(17).is_err(), "{mode}");
            assert_eq!(worker.queue_packets, 0);
            assert!(worker.failed && worker.exited);
            assert!(
                !timing.snapshot()["records"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|row| row["category"] == "worker_reported_elapsed")
            );
        }
    }

    #[test]
    fn ordered64_failed_argument_packing_has_a_host_span_but_no_worker_duration() {
        let timing = HostTiming::ordered64_diagnostic();
        let mut worker = worker("normal");
        worker.timing = Some(Box::new(WorkerTiming {
            timing: timing.clone(),
            rank: 0,
            pending: None,
        }));
        let dispatch = EngineeringTpDispatchV1 {
            kernel: "not_loaded",
            grid_workgroups: 1,
            workgroup_size: 64,
            arguments: vec![],
        };
        assert!(worker.submit_ordered_batch(&[dispatch]).is_err());
        let value = timing.snapshot();
        let rows = value["records"].as_array().unwrap();
        assert!(
            rows.iter()
                .any(|row| row["category"] == "span" && row["label"] == "ordered_dispatch_pack")
        );
        assert!(
            !rows
                .iter()
                .any(|row| row["category"] == "worker_reported_elapsed")
        );
        assert_eq!(worker.queue_packets, 0);
        assert!(worker.failed && worker.exited);
    }
}
