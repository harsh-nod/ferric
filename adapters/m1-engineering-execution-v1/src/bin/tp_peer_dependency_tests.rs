use super::super::{
    EngineeringTpBufferAccessV1 as Access, EngineeringTpRankTransportV1, PeerWorker,
};
use super::*;
use ferric_engine::tensor_parallel::Qwen3TensorParallelCollectiveKeyV1;
use ferric_m1_engineering_execution_v1::host_timing::HostTiming;
use ferric_spec::Qwen3ModelRole;
use std::cell::RefCell;
use std::process::{Command, Stdio};
use std::rc::Rc;

const FAKE: &str = r"
import json, os, struct, sys, time
mode = sys.argv[1]
def send(value, data=b''):
    header=json.dumps(value).encode()
    sys.stdout.buffer.write(struct.pack('<I',len(header))+header+data)
    sys.stdout.buffer.flush()
ready={'dependency_op':'ready','protocol':1,'mode':'device-peer-tp2-collective-dependency-v1',
 'target':'gfx950:xnack-','unique_ids':[1,2],'process_id':os.getpid(),'authority':'none',
 'producer_root':'ferric_qwen3_tp_mfma_gemm_partial_f32_v3',
 'consumer_root':'ferric_qwen3_tp_peer_tp2_ordered_residual_bf16_v18','currentness':'full',
 'control_allocation_flags':0x86000002,'collective_timeout_ms':2000}
if mode.startswith('ready_'):
    field=mode[6:]
    ready[field]={'protocol':4,'mode':'device-peer-serial-v4','target':'gfx942:xnack-',
        'unique_ids':[2,1],'process_id':0,'authority':'runtime','producer_root':'baseline',
        'consumer_root':'other','currentness':'operational','control_allocation_flags':0x84000004,
        'collective_timeout_ms':2001}[field]
send(ready)
cursors=[7,11]
while True:
    prefix=sys.stdin.buffer.read(4)
    if not prefix:break
    header=json.loads(sys.stdin.buffer.read(struct.unpack('<I',prefix)[0]))
    if header['dependency_op']=='rank':
        rank=header['request'];command=rank['command'];op=command['op']
        data=sys.stdin.buffer.read(command.get('payload_bytes',0))
        if op=='allocate': response={'op':'allocated','buffer':99,'bytes':command['bytes']}
        elif op=='configure_performance':response={'op':'performance_configured'}
        elif op=='close':response={'op':'closed'}
        else:response={'op':'error','message':'unsupported fixture ordinary command','fatal':True}
        send({'dependency_op':'rank','response':{'peer_op':'done','request':rank['request'],'rank':rank['rank'],'response':response}})
        if op=='close':break
        continue
    c=header['collective'];entries=c['producers']+c['consumers']
    assert len(entries)==4 and all(d['timeout_ms']==2000 for d in entries)
    data=sys.stdin.buffer.read(sum(d['payload_bytes'] for d in entries))
    assert len(data)==16 and data==bytes(range(16))
    if mode=='stall':time.sleep(60)
    if mode=='fatal':send({'dependency_op':'fatal','request_id':c['identity']['request_id'],'message':'terminal'});continue
    if mode=='partial':
        send({'dependency_op':'rank','response':{'peer_op':'done','request':c['identity']['request_id'],'rank':0,'response':{'op':'dispatched','elapsed_ns':1}}});continue
    receipt={'identity':c['identity'],'queues':[{'unique_id':r+1,'queue_epoch':0,'first_packet':first,'next_packet':first+3} for r,first in enumerate(cursors)],
        'kernel_counts':[2,2],'barrier_counts':[1,1],'packet_counts':[3,3],
        'completion_values':[[0,0,0],[0,0,0]],'final_frontiers':[[first+3]*2 for first in cursors]}
    if mode.startswith('identity_'):
        field=mode[9:];receipt['identity'][field]+=1
    if mode=='wrong_device':receipt['queues'].reverse()
    if mode=='nonzero':receipt['completion_values'][1][2]=1
    if mode=='undrained':receipt['final_frontiers'][1][1]-=1
    if mode=='count':receipt['kernel_counts'][1]=1
    if mode=='unknown':receipt['unexpected']=True
    if mode=='rollover':receipt['queues'][0]['queue_epoch']=1
    send({'dependency_op':'collective_completed','receipt':receipt})
    cursors=[x+3 for x in cursors]
";

fn connect(mode: &str) -> TpResult<Connection> {
    let child = Command::new("python3")
        .args(["-u", "-c", FAKE, mode])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    Connection::connect_mode(
        child,
        &[1, 2],
        Duration::from_millis(500),
        false,
        false,
        HostTiming::default(),
        true,
    )
}

fn slice(id: u64, elements: usize, element_bytes: u32, access: Access) -> Arg {
    Arg::Buffer {
        id,
        offset: 0,
        elements,
        element_bytes,
        access,
    }
}

fn request() -> Request {
    Request {
        key: Qwen3TensorParallelCollectiveKeyV1 {
            group_id: 0,
            model_role: Qwen3ModelRole::Target8B,
            epoch: 0,
            layer: 0,
            operation: Qwen3TensorParallelCollectiveV1::AttentionOutputSum,
        },
        rows: 1,
        producers: std::array::from_fn(|rank| EngineeringTpDispatchV1 {
            kernel: wire::PRODUCER_ROOT,
            grid_workgroups: 256,
            workgroup_size: 64,
            arguments: vec![
                slice(1 + rank as u64 * 3, 16 * 2048, 2, Access::Read),
                slice(2 + rank as u64 * 3, 4096 * 2048, 2, Access::Read),
                slice(3 + rank as u64 * 3, 16 * 4096, 4, Access::Write),
                Arg::U32(1),
                Arg::U32(4096),
                Arg::U32(2048),
                Arg::U32(2),
                Arg::U32(1),
            ],
        }),
        consumers: std::array::from_fn(|rank| {
            let mut arguments = vec![
                slice(3, 4096, 4, Access::Read),
                slice(6, 4096, 4, Access::Read),
            ];
            arguments.extend([slice(3, 0, 4, Access::Read); 6]);
            arguments.extend([
                slice(7 + rank as u64 * 2, 4096, 2, Access::Read),
                slice(8 + rank as u64 * 2, 4096, 2, Access::Write),
                Arg::U32(1),
                Arg::U32(2),
            ]);
            EngineeringTpDispatchV1 {
                kernel: wire::CONSUMER_ROOT,
                grid_workgroups: 64,
                workgroup_size: 64,
                arguments,
            }
        }),
    }
}

// These recording frames exercise transport atomicity, not compiler admission.
fn packed(connection: &Connection, request: &Request) -> wire::Collective {
    let dispatch = |grid| SequenceDispatchV1 {
        kernel: 1,
        payload_bytes: 4,
        workgroup: [64, 1, 1],
        grid: [grid, 1, 1],
        pointers: vec![],
        timeout_ms: 2000,
    };
    wire::Collective {
        identity: identity(
            request,
            connection.next_request,
            connection.dependency.as_ref().unwrap().next_generation,
        ),
        producers: [dispatch(16384), dispatch(16384)],
        consumers: [dispatch(4096), dispatch(4096)],
    }
}

#[test]
fn dependency_negotiation_requires_every_closed_policy_field() {
    for field in [
        "protocol",
        "mode",
        "target",
        "unique_ids",
        "process_id",
        "authority",
        "producer_root",
        "consumer_root",
        "currentness",
        "control_allocation_flags",
        "collective_timeout_ms",
    ] {
        assert!(connect(&format!("ready_{field}")).is_err(), "{field}");
    }
    let mut connection = connect("normal").unwrap();
    connection.terminate().unwrap();
    assert!(connection.exited);
}

#[test]
fn dependency_options_are_explicit_and_no_ambient_mode_is_inferred() {
    validate_options(&[1, 2], RuntimeOptions::default()).unwrap();
    validate_options(
        &[1, 2],
        RuntimeOptions {
            cache_admission: true,
            ..RuntimeOptions::default()
        },
    )
    .unwrap();
    for ids in [
        vec![1],
        vec![1, 1],
        vec![0, 2],
        vec![1, 2, 3, 4, 5, 6, 7, 8],
    ] {
        assert!(validate_options(&ids, RuntimeOptions::default()).is_err());
    }
    let changes: [fn(&mut RuntimeOptions); 7] = [
        |o| o.operational = true,
        |o| o.sequences = true,
        |o| o.ordered_batches = true,
        |o| o.full_forward = true,
        |o| o.rollover = true,
        |o| o.shared_full_currentness = true,
        |o| o.profile = true,
    ];
    for change in changes {
        let mut options = RuntimeOptions::default();
        change(&mut options);
        assert!(validate_options(&[1, 2], options).is_err());
    }
    let mut legacy = super::super::tests::fixture("normal");
    assert!(!legacy[0].supports_peer_dependency_collectives());
    assert!(
        legacy[0]
            .execute_peer_dependency_collective(&request())
            .is_err()
    );
    assert!(legacy[0].connection.borrow().failed);
}

#[test]
fn dependency_two_generations_allow_serial_setup_between_collectives() {
    let mut connection = connect("normal").unwrap();
    let request = request();
    for generation in 1..=2 {
        let collective = packed(&connection, &request);
        let receipt = transact(&mut connection, &request, collective, (0..16).collect()).unwrap();
        assert_eq!(receipt.generation, generation);
        assert_eq!(receipt.kernel_dispatches, [2; 2]);
        assert!(connection.pending.iter().all(Option::is_none));
        assert!(connection.completed.iter().all(Option::is_none));
        connection
            .sync(1, CommandV1::Allocate { bytes: 16 }, false, vec![])
            .unwrap();
    }
    assert_eq!(connection.next_request, 5);
    assert_eq!(connection.dependency.as_ref().unwrap().next_generation, 3);
    connection.sync(0, CommandV1::Close, false, vec![]).unwrap();
    connection.await_exit(true).unwrap();
}

#[test]
fn dependency_malformed_success_or_failure_never_commits_transport_generation() {
    for mode in [
        "identity_request_id",
        "identity_generation",
        "identity_group_id",
        "identity_epoch",
        "identity_layer",
        "wrong_device",
        "nonzero",
        "undrained",
        "count",
        "unknown",
        "rollover",
        "fatal",
        "partial",
        "stall",
    ] {
        let mut connection = connect(mode).unwrap();
        let request = request();
        let collective = packed(&connection, &request);
        assert!(
            transact(&mut connection, &request, collective, (0..16).collect()).is_err(),
            "{mode}"
        );
        assert!(connection.failed && connection.exited, "{mode}");
        assert_eq!(
            connection.dependency.as_ref().unwrap().next_generation,
            1,
            "{mode}"
        );
        assert_eq!(
            connection.dependency.as_ref().unwrap().last_frontiers,
            [0; 2],
            "{mode}"
        );
        assert!(connection.completed.iter().all(Option::is_none));
        assert!(execute(&mut connection, &request).is_err());
    }
}

#[test]
fn dependency_rank_one_cannot_execute_and_missing_kernel_is_terminal_before_send() {
    for rank in 0..2 {
        let connection = Rc::new(RefCell::new(connect("normal").unwrap()));
        let mut worker = PeerWorker {
            connection: Rc::clone(&connection),
            rank,
            kernels: BTreeMap::new(),
            pending: None,
            sequences: false,
        };
        assert!(worker.supports_peer_dependency_collectives());
        assert!(
            worker
                .execute_peer_dependency_collective(&request())
                .is_err()
        );
        assert_eq!(connection.borrow().next_request, 1);
        assert!(connection.borrow().failed && connection.borrow().exited);
        assert!(!worker.supports_peer_dependency_collectives());
    }
}

#[test]
fn dependency_allocation_bindings_require_full_capacity_owner_and_peer_visibility() {
    let mut connection = connect("normal").unwrap();
    let request = request();
    request.validate().unwrap();
    for rank in 0..2 {
        for arg in &request.producers[rank].arguments {
            if let Arg::Buffer {
                id,
                elements,
                element_bytes,
                ..
            } = arg
            {
                connection
                    .capacities
                    .insert(*id, elements * *element_bytes as usize);
                connection.ownership.insert(*id, (rank, true));
            }
        }
        for id in [7 + rank as u64 * 2, 8 + rank as u64 * 2] {
            connection.capacities.insert(id, 16 * 4096 * 2);
            connection.ownership.insert(id, (rank, true));
        }
    }
    for rank in 0..2 {
        validate_buffers(&connection, rank, &request.producers[rank], true).unwrap();
        validate_buffers(&connection, rank, &request.consumers[rank], false).unwrap();
    }
    connection.ownership.insert(3, (0, false));
    assert!(validate_buffers(&connection, 1, &request.consumers[1], false).is_err());
    connection.ownership.insert(3, (0, true));
    connection.capacities.insert(8, 4096 * 2);
    assert!(validate_buffers(&connection, 0, &request.consumers[0], false).is_err());
    connection.capacities.insert(8, 16 * 4096 * 2);
    connection.ownership.insert(4, (0, true));
    assert!(validate_buffers(&connection, 1, &request.producers[1], true).is_err());
    connection.terminate().unwrap();
}

#[test]
fn dependency_stale_frontier_and_key_are_rejected_before_public_receipt() {
    let request = request();
    let expected = identity(&request, 7, 3);
    let mut state = State::new([1, 2]);
    state.last_frontiers = [10, 14];
    let mut receipt = wire::Receipt {
        identity: expected,
        queues: [
            wire::RankQueue {
                unique_id: 1,
                queue_epoch: 0,
                first_packet: 10,
                next_packet: 13,
            },
            wire::RankQueue {
                unique_id: 2,
                queue_epoch: 0,
                first_packet: 14,
                next_packet: 17,
            },
        ],
        kernel_counts: [2; 2],
        barrier_counts: [1; 2],
        packet_counts: [3; 2],
        completion_values: [[0; 3]; 2],
        final_frontiers: [[13; 2], [17; 2]],
    };
    validate_receipt(&state, &request, expected, &receipt).unwrap();
    receipt.queues[0].first_packet = 7;
    receipt.queues[0].next_packet = 10;
    receipt.final_frontiers[0] = [10; 2];
    assert!(validate_receipt(&state, &request, expected, &receipt).is_err());
}

#[test]
fn dependency_collective_roots_cannot_be_sent_as_ordinary_rank_work() {
    for kernel in [
        wire::PRODUCER_ROOT,
        wire::CONSUMER_ROOT,
        "ferric_qwen3_tp_batch_gemm_partial_bf16_f32_v2",
        "ferric_qwen3_tp_wave_gemv_partial_f32_v3",
        "ferric_qwen3_tp_peer_ordered_residual_bf16_v4",
    ] {
        let connection = Rc::new(RefCell::new(connect("normal").unwrap()));
        let mut worker = PeerWorker {
            connection: Rc::clone(&connection),
            rank: 0,
            kernels: BTreeMap::new(),
            pending: None,
            sequences: false,
        };
        let dispatch = EngineeringTpDispatchV1 {
            kernel,
            grid_workgroups: 1,
            workgroup_size: 64,
            arguments: vec![],
        };
        let error = worker.submit(&dispatch).unwrap_err();
        assert!(error.contains("require the atomic request"));
        assert_eq!(connection.borrow().next_request, 1);
        assert!(connection.borrow().failed && connection.borrow().exited);
    }
}
