//! Distinct synthetic weight identities with bounded backing storage, CPU only.
use super::*;
use abi_fixture::{Access, Argument, Dispatch, Graph, Snapshot};

fn graph(position: u32, commands: &[EngineeringTpDispatchV1]) -> Graph {
    Graph {
        position,
        commands: commands
            .iter()
            .map(|command| Dispatch {
                kernel: command.kernel.into(),
                grid_workgroups: command.grid_workgroups,
                workgroup_size: command.workgroup_size,
                arguments: command
                    .arguments
                    .iter()
                    .map(|argument| match *argument {
                        EngineeringTpArgumentV1::Buffer {
                            id,
                            offset,
                            elements,
                            element_bytes,
                            access,
                        } => Argument::Buffer {
                            id,
                            offset,
                            elements,
                            element_bytes,
                            access: match access {
                                EngineeringTpBufferAccessV1::Read => Access::Read,
                                EngineeringTpBufferAccessV1::Write => Access::Write,
                                EngineeringTpBufferAccessV1::ReadWrite => Access::ReadWrite,
                            },
                        },
                        EngineeringTpArgumentV1::U32(value) => Argument::U32 { value },
                        EngineeringTpArgumentV1::F32(value) => Argument::F32 {
                            bits: value.to_bits(),
                        },
                    })
                    .collect(),
            })
            .collect(),
    }
}

#[test]
fn splitk_down_capacity_only_weights_have_no_byte_access() {
    let pool = token_pool();
    let mut driver = down_configured(&pool, true);
    let transport = &mut driver.inner.transports[0];
    assert_eq!(transport.readonly_capacities.len(), 73);
    let id = *transport.readonly_capacities.keys().next().unwrap();
    assert!(transport.read(id, 0, &mut [0; 2]).is_err());
    assert!(transport.write(id, 0, &[0; 2]).is_err());
    assert!(!transport.buffers.contains_key(&id));
    let view = EngineeringTpArgumentV1::Buffer {
        id,
        offset: 0,
        elements: 4096 * 12_288,
        element_bytes: 2,
        access: EngineeringTpBufferAccessV1::Read,
    };
    for mutation in 0..5 {
        let mut argument = view.clone();
        let EngineeringTpArgumentV1::Buffer {
            id,
            offset,
            elements,
            access,
            ..
        } = &mut argument
        else {
            unreachable!()
        };
        match mutation {
            0 => *access = EngineeringTpBufferAccessV1::Write,
            1 => *access = EngineeringTpBufferAccessV1::ReadWrite,
            2 => *offset = 1,
            3 => *elements = usize::MAX,
            4 => *id = u64::MAX,
            _ => unreachable!(),
        }
        assert!(
            transport
                .submit(&EngineeringTpDispatchV1 {
                    kernel: "capacity_fixture",
                    grid_workgroups: 1,
                    workgroup_size: 64,
                    arguments: vec![argument],
                })
                .is_err()
        );
    }
    assert!(transport.commands.is_empty() && transport.pending.is_none());
    assert!(transport.buffers.values().map(Vec::len).sum::<usize>() < 256 * 1024 * 1024);
    driver.close().unwrap();
}

#[test]
#[ignore = "requires fresh FERRIC_TOKEN_ABI_GRAPH_OUTPUT; distinct synthetic weights, CPU only"]
fn capture_down_native688_actual_driver_graphs() {
    capture(true);
}

#[test]
#[ignore = "requires fresh FERRIC_TOKEN_ABI_GRAPH_OUTPUT; same synthetic weight roster, CPU only"]
fn capture_down_control652_actual_driver_graphs() {
    capture(false);
}

fn capture(enabled: bool) {
    use std::io::Write as _;
    use std::os::unix::fs::OpenOptionsExt as _;
    let output =
        std::env::var_os("FERRIC_TOKEN_ABI_GRAPH_OUTPUT").expect("fresh explicit capture output");
    let output = std::path::Path::new(&output);
    assert!(output.is_absolute() && !output.exists());
    assert_eq!(
        output.parent().unwrap().canonicalize().unwrap(),
        output.parent().unwrap()
    );
    let mut pool = token_pool();
    let mut driver = down_configured(&pool, enabled);
    let mut identities = std::collections::BTreeSet::new();
    for layer in &driver.inner.ranks[0].layers {
        let original = layer.weight(Qwen3TensorKind::DownProjection);
        let kn = driver.projection.splitk_down_weight(0, original).unwrap();
        assert!(identities.insert(original.id) && identities.insert(kn.id));
        assert_eq!(
            driver.inner.transports[0].readonly_capacities[&original.id],
            100_663_296
        );
        assert_eq!(
            driver.inner.transports[0].readonly_capacities[&kn.id],
            100_663_296
        );
    }
    assert_eq!(identities.len(), 72);
    let sequence = pool
        .open_sequence(pool.scope(), &[1; 145], 0)
        .unwrap()
        .sequence();
    for first in (0..128).step_by(32) {
        let batch = reserve(&mut pool, sequence, first, first + 32);
        bind_down_rows(&mut driver, &batch, false, first == 96).unwrap();
        pool.begin_submission(&batch).unwrap();
        let result = driver
            .execute_selected(&batch, if first == 96 { &[31] } else { &[] })
            .unwrap();
        pool.commit_batch(&batch, result.completion).unwrap();
    }
    let mut graphs = Vec::new();
    let mut pages = Vec::new();
    for position in 128..145 {
        driver.inner.transports[0].events.borrow_mut().clear();
        let start = driver.inner.transports[0].commands.len();
        let batch = reserve(&mut pool, sequence, position, position + 1);
        bind_down_rows(&mut driver, &batch, true, true).unwrap();
        pool.begin_submission(&batch).unwrap();
        let result = driver.execute_selected(&batch, &[0]).unwrap();
        let transport = &driver.inner.transports[0];
        let commands = &transport.commands[start..];
        let count = if enabled { 688 } else { 652 };
        assert_eq!(commands.len(), count);
        for wait in [false, true] {
            assert_eq!(
                transport
                    .events
                    .borrow()
                    .iter()
                    .filter(|event| match event {
                        Event::TokenSubmit(_, n) => !wait && *n == count,
                        Event::TokenWait(_, n) => wait && *n == count,
                        _ => false,
                    })
                    .count(),
                1
            );
        }
        if position >= 143 {
            pages.push(scalar(&commands[8], 5));
            graphs.push(graph(position, commands));
        }
        pool.commit_batch(&batch, result.completion).unwrap();
    }
    assert_ne!(pages[0], pages[1]);
    assert_eq!(pool.committed_position(sequence).unwrap(), 145);
    pool.check_invariants().unwrap();
    let transport = &driver.inner.transports[0];
    let backed = transport.buffers.values().map(Vec::len).sum::<usize>()
        + transport
            .write_payloads
            .iter()
            .map(|(_, _, bytes)| bytes.len())
            .sum::<usize>();
    assert!(backed < 256 * 1024 * 1024);
    let buffers = transport
        .buffers
        .iter()
        .map(|(&id, bytes)| (id, bytes.len()))
        .chain(
            transport
                .readonly_capacities
                .iter()
                .map(|(&id, &size)| (id, size)),
        )
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        buffers.len(),
        transport.buffers.len() + transport.readonly_capacities.len()
    );
    let snapshot = Snapshot {
        schema: abi_fixture::SCHEMA.into(),
        buffers: buffers.into_iter().collect(),
        graphs,
    };
    let bytes = serde_json::to_vec(&snapshot).unwrap();
    assert!(bytes.len() <= abi_fixture::MAX_BYTES);
    driver.close().unwrap();
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(output)
        .unwrap();
    file.write_all(&bytes).unwrap();
    file.sync_all().unwrap();
    println!(
        "{}",
        serde_json::json!({"schema":"FerricNativeDownCaptureR1", "enabled":enabled,
        "graphs":2,"positions":[143,144],"distinct_down_weight_ids":72,
        "weight_contents":"capacity-only synthetic identities, not authenticated model bytes",
        "backed_buffer_and_write_payload_bytes":backed,"capture_bytes":bytes.len(),"native_executed":false})
    );
}
