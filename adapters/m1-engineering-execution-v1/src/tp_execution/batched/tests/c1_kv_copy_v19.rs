//! Recording-transport routing and byte-copy checks, not native GPU measurements.

use super::super::c1_kv_copy_v19::{CopySlotV19, admit};
use super::*;
use crate::tp_artifact::{
    C1KvCopyBindingV19, Fp32ArgmaxBindingV11, QueryHoistBindingV14, WaveRmsNormBindingV15,
};

const COPY: &str = crate::tp_artifact::ENGINEERING_TP_C1_KV_COPY_EXPORTS_V19[0];
const LEGACY: &str = "ferric_qwen3_tp_batch32_paged_kv_append_v5";

fn configured(pool: &EngineeringTpPagedPoolV1) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = super::wave_rmsnorm_v15::configured(pool);
    driver.admitted_query_hoist_v14 = Some(QueryHoistBindingV14::recording());
    driver.admitted_c1_kv_copy_v19 = Some(C1KvCopyBindingV19::recording());
    driver
}

fn select(driver: &mut EngineeringTpBatchExecutionV2<Recording>, enabled: bool) -> TpResult<()> {
    driver.configure_c1_copy_bindings_v19(
        Fp32ArgmaxBindingV11::recording(),
        QueryHoistBindingV14::recording(),
        WaveRmsNormBindingV15::recording(),
        C1KvCopyBindingV19::recording(),
        enabled,
    )
}

#[test]
fn c1_copy_v19_changes_only_single_row_append_and_preserves_counts_and_multirow_prefill() {
    for rows in [1, 2, 16, 17, 32] {
        let mut reference = None;
        for enabled in [false, true] {
            let mut pool = wide_pool();
            let mut driver = configured(&pool);
            select(&mut driver, enabled).unwrap();
            assert_eq!(
                driver.kv_append_mode(),
                if enabled {
                    "parallel-c1-v19"
                } else {
                    "baseline"
                }
            );
            assert_eq!(
                (driver.attention_mode(), driver.rmsnorm_mode()),
                ("query-hoist-v14", "wave-v15")
            );
            let before_allocations = driver.inner.transports[0]
                .buffers
                .keys()
                .copied()
                .collect::<Vec<_>>();
            let batch = prepare(&mut pool, rows);
            pool.begin_submission(&batch).unwrap();
            let output = driver
                .execute_selected(&batch, &[rows as usize - 1])
                .unwrap();
            let transport = &driver.inner.transports[0];
            assert_eq!(driver.dispatch_counts(), [616]);
            let copies = transport
                .commands
                .iter()
                .filter(|command| command.kernel == COPY)
                .count();
            assert_eq!(copies, if enabled && rows == 1 { 36 } else { 0 });
            assert_eq!(
                transport
                    .commands
                    .iter()
                    .filter(|command| command.kernel == LEGACY)
                    .count(),
                36 - copies
            );
            let nonappend = transport
                .commands
                .iter()
                .filter(|command| !matches!(command.kernel, COPY | LEGACY))
                .cloned()
                .collect::<Vec<_>>();
            let observed = (
                nonappend,
                output.choices.clone(),
                transport.reads.clone(),
                transport.write_payloads.clone(),
                transport.packet_preparations.clone(),
            );
            if let Some(expected) = &reference {
                assert_eq!(&observed, expected);
            } else {
                reference = Some(observed);
            }
            assert_eq!(
                transport.buffers.keys().copied().collect::<Vec<_>>(),
                before_allocations
            );
            pool.commit_batch(&batch, output.completion).unwrap();
            pool.check_invariants().unwrap();
            driver.close().unwrap();
        }
    }
}

#[test]
fn c1_copy_v19_checked_subviews_copy_bits_and_leave_all_adjacent_slots_unchanged() {
    let mut pool = wide_pool();
    let mut driver = configured(&pool);
    let batch = prepare(&mut pool, 32);
    for row_index in [0, 15, 16, 31] {
        let row = &batch.rows()[row_index];
        let slot = CopySlotV19::new(row, 4, 64).unwrap();
        let command = slot.command(&driver.inner.ranks[0], 0);
        let command = super::super::super::row_profile::bind_storage(32, false, command).unwrap();
        let transport = &mut driver.inner.transports[0];
        for (source, destination) in [(0, 2), (1, 3)] {
            let input = buffer(&command, source);
            let output = buffer(&command, destination);
            let pattern = (0..1024_usize)
                .flat_map(|index| {
                    u16::try_from((index * 67 + source * 31) & 0xffff)
                        .unwrap()
                        .to_le_bytes()
                })
                .collect::<Vec<_>>();
            transport.buffers.get_mut(&input.0).unwrap()[..2048].copy_from_slice(&pattern);
            transport.buffers.get_mut(&output.0).unwrap().fill(0xa5);
        }
        transport.submit(&command).unwrap();
        transport.wait().unwrap();
        for (source, destination) in [(0, 2), (1, 3)] {
            let input = buffer(&command, source);
            let output = buffer(&command, destination);
            let bytes = &transport.buffers[&output.0];
            assert_eq!(
                &bytes[output.1..output.1 + 2048],
                &transport.buffers[&input.0][..2048]
            );
            assert!(
                bytes[..output.1]
                    .iter()
                    .chain(&bytes[output.1 + 2048..])
                    .all(|&byte| byte == 0xa5)
            );
            let expected_slot = row.writable_physical_page() * 16 + row.writable_token_offset();
            assert_eq!(output.1, expected_slot as usize * 2048);
        }
        for (pages, context) in [(0, 64), (513, 64), (4, 0), (4, 8193), (4, row.position())] {
            assert!(CopySlotV19::new(row, pages, context).is_err());
        }
    }
}

#[test]
fn c1_copy_v19_admission_requires_loaded_identity_before_allocations() {
    let binding = C1KvCopyBindingV19::recording();
    let mut driver = configured(&wide_pool());
    let transport = &mut driver.inner.transports[0];
    assert!(admit(std::slice::from_mut(transport), binding).is_err());
    transport.buffers.clear();
    assert!(admit(std::slice::from_mut(transport), binding).is_err());
    transport.c1_kv_copy_v19_loaded = Some(binding.hsaco);
    admit(std::slice::from_mut(transport), binding).unwrap();
    transport.argmax_peer = Some((1, 0, 1));
    assert!(admit(std::slice::from_mut(transport), binding).is_err());
    assert!(transport.commands.is_empty());
    assert!(transport.buffers.is_empty());
}

#[test]
fn c1_copy_v19_preserves_cached_immutable_prefix_and_writes_only_the_exclusive_suffix_slot() {
    let mut pool = wide_pool();
    let mut driver = configured(&pool);
    select(&mut driver, true).unwrap();
    let first = prepare(&mut pool, 16);
    let sequence = first.rows()[0].sequence();
    let prefix_page = first.rows()[0].writable_physical_page();
    pool.begin_submission(&first).unwrap();
    let output = driver.execute_selected(&first, &[15]).unwrap();
    pool.commit_batch(&first, output.completion).unwrap();
    let caches = driver.inner.ranks[0]
        .layers
        .iter()
        .flat_map(|layer| [layer.k_cache.id, layer.v_cache.id])
        .collect::<Vec<_>>();
    let prefix_begin = prefix_page as usize * 16 * 2048;
    for id in &caches {
        driver.inner.transports[0].buffers.get_mut(id).unwrap()
            [prefix_begin..prefix_begin + 16 * 2048]
            .fill(0x5a);
    }
    pool.retire_sequence(sequence, true, 1).unwrap();
    let prompt = (0..17).collect::<Vec<_>>();
    let hit = pool.open_sequence(pool.scope(), &prompt, 2).unwrap();
    assert_eq!(hit.hit_tokens(), 16);
    assert_eq!(hit.physical_pages(), &[prefix_page]);
    let batch = pool
        .reserve_batch(&[EngineeringTpPageRowV1 {
            sequence: hit.sequence(),
            token: 16,
            position: 16,
        }])
        .unwrap();
    let row = &batch.rows()[0];
    assert_ne!(row.writable_physical_page(), prefix_page);
    let offset = (row.writable_physical_page() * 16 + row.writable_token_offset()) as usize * 2048;
    let expected = caches
        .iter()
        .map(|id| {
            let mut bytes = driver.inner.transports[0].buffers[id].clone();
            // The recording transport's QKV/RoPE writers produce zero words.
            bytes[offset..offset + 2048].fill(0);
            (*id, bytes)
        })
        .collect::<Vec<_>>();
    pool.begin_submission(&batch).unwrap();
    let output = driver.execute_selected(&batch, &[0]).unwrap();
    for (id, bytes) in expected {
        assert_eq!(driver.inner.transports[0].buffers[&id], bytes);
    }
    assert_eq!(
        driver.inner.transports[0]
            .commands
            .iter()
            .filter(|command| command.kernel == COPY)
            .count(),
        36
    );
    pool.commit_batch(&batch, output.completion).unwrap();
    pool.check_invariants().unwrap();
    driver.close().unwrap();
}

#[test]
fn c1_copy_v19_incompatible_binding_and_geometry_reject_before_selection() {
    for mutation in 0..13 {
        let mut driver = configured(&wide_pool());
        match mutation {
            0 => driver.admitted_c1_kv_copy_v19 = None,
            1 => {
                let mut binding = C1KvCopyBindingV19::recording();
                binding.hsaco[0] ^= 1;
                driver.admitted_c1_kv_copy_v19 = Some(binding);
            }
            2 => driver.row_capacity = 16,
            3 => driver.inner.large_kv = true,
            4 => driver.physical_pages = 0,
            5 => driver.physical_pages = 513,
            6 => driver.context_tokens = 8193,
            7 => driver.inner.ranks[0].k_rotated.elements -= 1,
            8 => driver.inner.ranks[0].v.element_bytes = 4,
            9 => driver.inner.ranks[0].layers[0].k_cache.elements -= 1,
            10 => driver.inner.ranks[0].layers[35].v_cache.element_bytes = 4,
            11 => driver.last_batch = 1,
            12 => driver.inner.closed = true,
            _ => unreachable!(),
        }
        assert!(select(&mut driver, true).is_err(), "mutation {mutation}");
        assert!(driver.c1_kv_copy_v19.is_none());
        assert!(driver.query_hoist_v14.is_none());
        assert!(driver.wave_rmsnorm_v15.is_none());
        assert!(driver.inner.ordered_batches.is_none());
        assert!(driver.inner.transports[0].commands.is_empty());
    }
    for enabled in [false, true] {
        let mut driver = configured(&wide_pool());
        select(&mut driver, enabled).unwrap();
        assert!(select(&mut driver, false).is_err());
        assert!(select(&mut driver, true).is_err());
    }
}

#[test]
fn c1_copy_v19_foreign_stale_batches_and_copy_failures_cannot_commit() {
    for failure in [
        Failure::KvCopySubmit,
        Failure::KvCopyWait,
        Failure::OrderedSubmit,
        Failure::OrderedWait,
    ] {
        let mut pool = wide_pool();
        let mut driver = configured(&pool);
        select(&mut driver, true).unwrap();
        driver.inner.transports[0].failure = Some(failure);
        let batch = prepare(&mut pool, 1);
        pool.begin_submission(&batch).unwrap();
        assert!(driver.execute_selected(&batch, &[0]).is_err());
        assert!(driver.poisoned);
        assert_eq!(driver.completed_batches(), 0);
        assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
        assert!(driver.inner.transports[0].pending.is_none());
        assert!(driver.inner.transports[0].pending_ordered.is_none());
        pool.quarantine_batch(&batch).unwrap();
        driver.close().unwrap();
    }
    let mut pool = wide_pool();
    let mut foreign = wide_pool();
    let mut driver = configured(&pool);
    select(&mut driver, true).unwrap();
    assert!(
        driver
            .execute_selected(&prepare(&mut foreign, 1), &[0])
            .is_err()
    );
    assert!(driver.inner.transports[0].commands.is_empty());
    let batch = prepare(&mut pool, 1);
    pool.begin_submission(&batch).unwrap();
    let output = driver.execute_selected(&batch, &[0]).unwrap();
    pool.commit_batch(&batch, output.completion).unwrap();
    let before = driver.inner.transports[0].commands.clone();
    assert!(driver.execute_selected(&batch, &[0]).is_err());
    assert_eq!(driver.inner.transports[0].commands, before);
}
