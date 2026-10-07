//! Opt-in sidecar. The existing send/wait boundary owns all real publication.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use ferric_m1_engineering_execution_v1::model_timestamps::{
    BatchIdentity, Collector, KernelIdentity, MAX_GROUP_PACKETS, MAX_ORDERED64_GROUP_PACKETS,
    PacketTag,
};

use super::{CommandV1, Incoming, PendingRequest, ResponseV1, TpResult, Worker};

pub(super) type SharedCollector = Rc<RefCell<Option<Collector>>>;

pub(super) struct State {
    collector: SharedCollector,
    tags: VecDeque<PacketTag>,
    rollover_pending: bool,
}

impl State {
    fn apply<T>(
        &self,
        action: impl FnOnce(&mut Collector) -> Result<T, &'static str>,
    ) -> TpResult<T> {
        let mut held = self.collector.borrow_mut();
        let collector = held
            .as_mut()
            .ok_or("model timestamp collector was already consumed")?;
        action(collector).map_err(str::to_owned)
    }
}

impl Worker {
    #[allow(dead_code)] // Other controllers share this module without opting in.
    pub(crate) fn enable_model_timestamps(&mut self, layers: u32) -> TpResult<SharedCollector> {
        if self.failed
            || self.exited
            || self.pending.is_some()
            || self.model_timestamps.is_some()
            || !self.buffers.is_empty()
            || self.queue_packets != 0
            || self.queue_epoch != 0
            || self.options.sequences
            || !self.options.ordered_batches
            || self.options.ordered64 != self.options.ordered64_packet_ticks
            || self.options.validate_ordered64().is_err()
            || self.diagnostic_identity.1 != 0
        {
            return self.reject("model timestamps require a fresh explicitly selected TP1 worker");
        }
        let mut collector = if self.options.ordered64_packet_ticks {
            Collector::new_ordered64(self.diagnostic_identity.0, layers)?
        } else {
            Collector::new(self.diagnostic_identity.0, layers)?
        };
        for (symbol, loaded) in &self.kernels {
            collector.register_kernel(KernelIdentity {
                kernel: loaded.id,
                symbol: symbol.clone(),
                object_sha256: loaded.image,
            })?;
        }
        let shared = Rc::new(RefCell::new(Some(collector)));
        self.model_timestamps = Some(Box::new(State {
            collector: Rc::clone(&shared),
            tags: VecDeque::new(),
            rollover_pending: false,
        }));
        Ok(shared)
    }

    pub(super) fn bind_model_timestamp_batch(&mut self, batch: BatchIdentity) -> TpResult<()> {
        let Some(state) = &self.model_timestamps else {
            return Ok(());
        };
        if !state.tags.is_empty() || state.rollover_pending || self.pending.is_some() {
            return self.reject("model timestamp batch overlaps pending attribution");
        }
        if let Err(error) = state.apply(|collector| collector.begin_batch(batch)) {
            return self.reject(error);
        }
        Ok(())
    }

    pub(super) fn enqueue_model_timestamp_tag(&mut self, tag: PacketTag) -> TpResult<()> {
        let Some(state) = &mut self.model_timestamps else {
            return Ok(());
        };
        if self.failed
            || self.exited
            || self.pending.is_some()
            || state.tags.len()
                > if self.options.ordered64_packet_ticks {
                    MAX_ORDERED64_GROUP_PACKETS
                } else {
                    MAX_GROUP_PACKETS
                }
            || state.rollover_pending
        {
            return self.reject("model timestamp semantic queue is unavailable or full");
        }
        state.tags.push_back(tag);
        Ok(())
    }

    pub(super) fn profile_model_header(
        &mut self,
        header: CommandV1,
        payload: &[u8],
    ) -> TpResult<CommandV1> {
        let Some(state) = &mut self.model_timestamps else {
            return Ok(header);
        };
        let count = match &header {
            CommandV1::Dispatch { .. } => Some(1),
            CommandV1::DispatchOrderedBatch { dispatches, .. } => Some(dispatches.len()),
            CommandV1::DispatchOrderedBatch64 { dispatches, .. }
                if self.options.ordered64_packet_ticks =>
            {
                Some(dispatches.len())
            }
            CommandV1::DispatchSequence { .. }
            | CommandV1::DispatchOrderedBatch64 { .. }
            | CommandV1::DispatchOrderedBatch64Profiled { .. } => {
                return Err("model diagnostic cannot regroup or substitute a sequence".into());
            }
            CommandV1::RolloverQueue { .. } => {
                if !state.tags.is_empty() || state.rollover_pending {
                    return Err("model rollover has deferred command attribution".into());
                }
                let checked = state.apply(Collector::begin_rollover)?;
                if checked != header {
                    return Err("model and worker rollover frontiers differ".into());
                }
                state.rollover_pending = true;
                return Ok(header);
            }
            CommandV1::Close => {
                if !state.tags.is_empty() || state.rollover_pending {
                    return Err("model close has unconsumed semantic tags".into());
                }
                None
            }
            CommandV1::LoadKernel { .. } => {
                return Err("model diagnostic kernel roster is immutable".into());
            }
            _ => None,
        };
        let Some(count) = count else {
            return Ok(header);
        };
        let maximum = if self.options.ordered64_packet_ticks {
            MAX_ORDERED64_GROUP_PACKETS
        } else {
            MAX_GROUP_PACKETS
        };
        if !(1..=maximum).contains(&count) || state.tags.len() < count {
            return Err("model publication lacks exact bounded command attribution".into());
        }
        let tags = state.tags.drain(..count).collect();
        state.apply(|collector| collector.begin_group(header, tags, payload))
    }

    pub(super) fn validate_model_response(&mut self, incoming: Incoming) -> TpResult<Incoming> {
        let Some(state) = &mut self.model_timestamps else {
            return Ok(incoming);
        };
        if state.rollover_pending {
            state.apply(|collector| {
                collector.complete_rollover(&incoming.header, &incoming.payload)
            })?;
            state.rollover_pending = false;
            return Ok(incoming);
        }
        let count = match self.pending {
            Some(PendingRequest::Dispatch) => Some((true, 1)),
            Some(PendingRequest::OrderedBatch(count)) => Some((false, count)),
            #[cfg(feature = "c1-ordered64")]
            Some(PendingRequest::OrderedBatch64(count)) if self.options.ordered64_packet_ticks => {
                Some((false, count))
            }
            _ => None,
        };
        let Some((single, count)) = count else {
            if matches!(
                incoming.header,
                ResponseV1::DispatchOrderedBatch64ProfiledCompleted { .. }
            ) {
                return Err("unsolicited model timestamps".into());
            }
            return Ok(incoming);
        };
        state.apply(|collector| collector.complete_group(&incoming.header, &incoming.payload))?;
        let ResponseV1::DispatchOrderedBatch64ProfiledCompleted { elapsed_ns, .. } =
            incoming.header
        else {
            return Err("validated model timestamp response changed kind".into());
        };
        // Preserve the original wait contract only after validating every raw
        // record. Existing queue/rank counters still commit in their old wait.
        Ok(Incoming {
            header: if single {
                ResponseV1::Dispatched { elapsed_ns }
            } else if self.options.ordered64_packet_ticks {
                ResponseV1::DispatchOrderedBatch64Completed {
                    completed_dispatches: u32::try_from(count)
                        .map_err(|_| "model ordered64 group count conversion")?,
                    elapsed_ns,
                }
            } else {
                ResponseV1::DispatchOrderedBatchCompleted {
                    completed_dispatches: u32::try_from(count)
                        .map_err(|_| "model group count conversion")?,
                    elapsed_ns,
                }
            },
            payload: Vec::new(),
        })
    }

    pub(super) fn poison_model_timestamps(&mut self) {
        if let Some(state) = &mut self.model_timestamps {
            if let Some(collector) = state.collector.borrow_mut().as_mut() {
                collector.poison();
            }
            state.tags.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tp_worker::{EngineeringTpRankTransportV1, RuntimeOptions, wire};
    use ferric_m1_engineering_execution_v1::model_timestamps::Operation;
    use std::process::{Command, Stdio};
    use std::time::Duration;

    const FAKE: &str = r"
import json, struct, sys
mode = sys.argv[1]
limit = int(sys.argv[2])
def send(value):
    data = json.dumps(value).encode()
    sys.stdout.buffer.write(struct.pack('<I', len(data)) + data)
    sys.stdout.buffer.flush()
send({'op':'ready','protocol':1,'target':'gfx950:xnack-', 'device_unique_id':1,'authority':'none'})
epoch = packets = 0
while True:
    prefix = sys.stdin.buffer.read(4)
    if len(prefix) != 4: sys.exit(3)
    header = json.loads(sys.stdin.buffer.read(struct.unpack('<I', prefix)[0]))
    op = header['op']
    if op == 'close':
        send({'op':'closed'})
        sys.exit(0)
    if op == 'configure_performance':
        assert limit == 64 and header['profile'] is False
        assert header['cache_kernel_admission'] and header['operational_currentness']
        send({'op':'performance_configured'})
        continue
    if op == 'rollover_queue':
        assert header['expected_epoch'] == epoch and header['expected_completed_packets'] == packets
        epoch += 1
        send({'op':'queue_rolled_over','retired_packets':packets,'queue_epoch':epoch})
        packets = 0
        continue
    assert op == 'dispatch_ordered_batch64_profiled'
    entries = header['dispatches']
    assert 1 <= len(entries) <= limit and header['timeout_ms'] == 60000
    size = sum(entry['payload_bytes'] for entry in entries)
    assert sys.stdin.buffer.read(size) == bytes(size)
    stamps = [{'packet_id':packets+i,'kernel':entry['kernel'],'start_tick':100+i,'end_tick':120+i}
              for i,entry in enumerate(entries)]
    if mode == 'kernel': stamps[-1]['kernel'] += 1
    if mode == 'zero': stamps[-1]['start_tick'] = 0
    if mode == 'short': stamps.pop()
    reply = {'op':'dispatch_ordered_batch64_profiled_completed','device_unique_id':1,
             'queue_epoch':epoch,'elapsed_ns':7,'timestamps':stamps}
    if mode == 'device': reply['device_unique_id'] = 2
    if mode == 'legacy': reply = {'op':'dispatched','elapsed_ns':7}
    send(reply)
    packets += len(entries)
";

    fn worker(mode: &str, expected: u64) -> (Worker, SharedCollector) {
        worker_mode(mode, expected, false)
    }

    fn worker_mode(mode: &str, expected: u64, wide: bool) -> (Worker, SharedCollector) {
        let child = Command::new("python3")
            .args(["-u", "-c", FAKE, mode, if wide { "64" } else { "16" }])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut worker = Worker::connect(child, 1, Duration::from_secs(2)).unwrap();
        let options = RuntimeOptions {
            cache_admission: wide,
            operational: wide,
            ordered_batches: true,
            ordered64: wide,
            ordered64_packet_ticks: wide,
            rollover: true,
            ..RuntimeOptions::default()
        };
        worker.configure_options(options).unwrap();
        let mut collector = if wide {
            Collector::new_ordered64(1, 36).unwrap()
        } else {
            Collector::new(1, 36).unwrap()
        };
        collector
            .register_kernel(KernelIdentity {
                kernel: 99,
                symbol: "fake_abi_not_native".into(),
                object_sha256: [1; 32],
            })
            .unwrap();
        let shared = Rc::new(RefCell::new(Some(collector)));
        worker.model_timestamps = Some(Box::new(State {
            collector: Rc::clone(&shared),
            tags: VecDeque::new(),
            rollover_pending: false,
        }));
        worker
            .bind_model_timestamp_batch(BatchIdentity {
                ordinal: 1,
                scheduler_batch_id: 1,
                request: [0, 1],
                prefill_rows: 0,
                decode_rows: 1,
                expected_packets: expected,
            })
            .unwrap();
        (worker, shared)
    }

    fn publish(worker: &mut Worker, count: usize, single: bool) -> TpResult<()> {
        for _ in 0..count {
            worker.enqueue_model_timestamp_tag(PacketTag {
                batch_ordinal: 1,
                layer: Some(0),
                operation: Operation::QueryProjection,
            })?;
        }
        let mut command = original(count, single);
        if !single && worker.options.ordered64_packet_ticks {
            let CommandV1::DispatchOrderedBatch {
                dispatches,
                timeout_ms,
            } = command
            else {
                return Err("unexpected fake original command".into());
            };
            command = CommandV1::DispatchOrderedBatch64 {
                dispatches,
                timeout_ms,
            };
        }
        worker.send(command, vec![0; count * 4])
    }

    fn original(count: usize, single: bool) -> CommandV1 {
        let entry = wire::OrderedBatchDispatchV1 {
            kernel: 99,
            payload_bytes: 4,
            workgroup: [64, 1, 1],
            grid: [64, 1, 1],
            pointers: vec![],
        };
        if single {
            CommandV1::Dispatch {
                kernel: entry.kernel,
                payload_bytes: entry.payload_bytes,
                workgroup: entry.workgroup,
                grid: entry.grid,
                pointers: entry.pointers,
                timeout_ms: 60_000,
            }
        } else {
            CommandV1::DispatchOrderedBatch {
                dispatches: vec![entry; count],
                timeout_ms: 60_000,
            }
        }
    }

    #[test]
    fn deferred_group_drain_leaves_the_next_singletons_semantic_tag_untouched() {
        let (mut worker, shared) = worker("valid", 17);
        for _ in 0..16 {
            worker
                .enqueue_model_timestamp_tag(PacketTag {
                    batch_ordinal: 1,
                    layer: Some(0),
                    operation: Operation::QueryProjection,
                })
                .unwrap();
        }
        worker
            .enqueue_model_timestamp_tag(PacketTag {
                batch_ordinal: 1,
                layer: None,
                operation: Operation::Embedding,
            })
            .unwrap();
        worker.send(original(16, false), vec![0; 64]).unwrap();
        worker.wait_ordered_batch(16).unwrap();
        assert_eq!(worker.model_timestamps.as_ref().unwrap().tags.len(), 1);
        worker.send(original(1, true), vec![0; 4]).unwrap();
        worker.wait().unwrap();
        worker.close().unwrap();
        let capture = shared.borrow_mut().take().unwrap().finish(17).unwrap();
        let value: serde_json::Value =
            serde_json::from_slice(&capture.to_bounded_json().unwrap()).unwrap();
        assert_eq!(value["groups"], serde_json::json!([[0, 1, 16], [1, 0, 1]]));
        assert!(
            value["records"].as_array().unwrap()[..16]
                .iter()
                .all(|row| row[3] == 2)
        );
        assert_eq!(value["records"][16][3], 0);
    }

    #[test]
    fn typed_reader_sidecar_keeps_single_ordered16_and_rollover_wait_contracts() {
        let (mut worker, shared) = worker("valid", 18);
        publish(&mut worker, 1, true).unwrap();
        worker.wait().unwrap();
        publish(&mut worker, 16, false).unwrap();
        worker.wait_ordered_batch(16).unwrap();
        assert_eq!(worker.queue_packets, 17);
        worker
            .prepare_packets(wire::MAX_UNRETIRED_RING_PACKETS_V1)
            .unwrap();
        assert_eq!((worker.queue_epoch, worker.queue_packets), (1, 0));
        publish(&mut worker, 1, true).unwrap();
        worker.wait().unwrap();
        worker.close().unwrap();
        assert!(worker.exited);
        let capture = shared.borrow_mut().take().unwrap().finish(18).unwrap();
        let value: serde_json::Value =
            serde_json::from_slice(&capture.to_bounded_json().unwrap()).unwrap();
        assert_eq!(
            value["groups"],
            serde_json::json!([[0, 0, 1], [1, 1, 16], [2, 0, 1]])
        );
        assert_eq!(value["records"].as_array().unwrap().len(), 18);
    }

    #[test]
    #[cfg(feature = "c1-ordered64")]
    fn ordered64_typed_reader_keeps_original_groups_rollover_and_close() {
        let (mut worker, shared) = worker_mode("valid", 77, true);
        publish(&mut worker, 1, true).unwrap();
        worker.wait().unwrap();
        publish(&mut worker, 64, false).unwrap();
        worker.wait_ordered_batch(64).unwrap();
        assert_eq!(worker.queue_packets, 65);
        worker
            .prepare_packets(wire::MAX_UNRETIRED_RING_PACKETS_V1)
            .unwrap();
        assert_eq!((worker.queue_epoch, worker.queue_packets), (1, 0));
        publish(&mut worker, 12, false).unwrap();
        worker.wait_ordered_batch(12).unwrap();
        worker.close().unwrap();
        assert!(worker.exited && !worker.failed);
        let capture = shared.borrow_mut().take().unwrap().finish(77).unwrap();
        let value: serde_json::Value =
            serde_json::from_slice(&capture.to_bounded_json().unwrap()).unwrap();
        assert_eq!(
            value["groups"],
            serde_json::json!([[0, 0, 1], [1, 2, 64], [2, 2, 12]])
        );
        assert_eq!(value["records"].as_array().unwrap().len(), 77);
    }

    #[test]
    #[cfg(feature = "c1-ordered64")]
    fn ordered64_full_group_keeps_next_packets_lookahead_tag() {
        let (mut worker, shared) = worker_mode("valid", 65, true);
        for _ in 0..64 {
            worker
                .enqueue_model_timestamp_tag(PacketTag {
                    batch_ordinal: 1,
                    layer: Some(0),
                    operation: Operation::QueryProjection,
                })
                .unwrap();
        }
        worker
            .enqueue_model_timestamp_tag(PacketTag {
                batch_ordinal: 1,
                layer: None,
                operation: Operation::Embedding,
            })
            .unwrap();
        let CommandV1::DispatchOrderedBatch {
            dispatches,
            timeout_ms,
        } = original(64, false)
        else {
            panic!("original group")
        };
        worker
            .send(
                CommandV1::DispatchOrderedBatch64 {
                    dispatches,
                    timeout_ms,
                },
                vec![0; 256],
            )
            .unwrap();
        worker.wait_ordered_batch(64).unwrap();
        assert_eq!(worker.model_timestamps.as_ref().unwrap().tags.len(), 1);
        worker.send(original(1, true), vec![0; 4]).unwrap();
        worker.wait().unwrap();
        worker.close().unwrap();
        let capture = shared.borrow_mut().take().unwrap().finish(65).unwrap();
        let value: serde_json::Value =
            serde_json::from_slice(&capture.to_bounded_json().unwrap()).unwrap();
        assert_eq!(value["groups"], serde_json::json!([[0, 2, 64], [1, 0, 1]]));
        assert_eq!(value["records"][64][3], 0);
    }

    #[test]
    #[cfg(feature = "c1-ordered64")]
    fn ordered64_malformed_tail_poison_retains_uncommitted_frontier() {
        for mode in ["kernel", "zero", "short", "device", "legacy"] {
            let (mut worker, shared) = worker_mode(mode, 64, true);
            publish(&mut worker, 64, false).unwrap();
            assert!(worker.wait_ordered_batch(64).is_err(), "{mode}");
            assert_eq!((worker.queue_epoch, worker.queue_packets), (0, 0));
            assert!(worker.failed && worker.exited);
            assert!(shared.borrow_mut().take().unwrap().finish(64).is_err());
        }
    }

    #[test]
    fn malformed_timestamps_poison_without_advancing_original_worker_frontier() {
        for mode in ["kernel", "zero", "short", "device", "legacy"] {
            let (mut worker, shared) = worker(mode, 1);
            publish(&mut worker, 1, true).unwrap();
            assert!(worker.wait().is_err(), "{mode}");
            assert_eq!(worker.queue_packets, 0);
            assert!(worker.failed && worker.exited);
            assert!(shared.borrow_mut().take().unwrap().finish(1).is_err());
        }
    }

    #[test]
    fn missing_semantic_tag_rejects_before_publication_and_reaps_only_owned_worker() {
        let (mut worker, shared) = worker("valid", 1);
        let command = CommandV1::Dispatch {
            kernel: 99,
            payload_bytes: 0,
            workgroup: [64, 1, 1],
            grid: [64, 1, 1],
            pointers: vec![],
            timeout_ms: 60_000,
        };
        assert!(worker.send(command, vec![]).is_err());
        assert!(worker.failed && worker.exited);
        assert!(shared.borrow_mut().take().unwrap().finish(0).is_err());
    }
}
