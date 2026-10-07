use super::*;

#[derive(Default)]
struct Sparse {
    image: Option<[u8; 32]>,
    binding_checks: usize,
    allocations: Vec<usize>,
    peer: bool,
}

impl EngineeringTpRankTransportV1 for Sparse {
    fn require_loaded_image(&mut self, image: [u8; 32], kernels: &[&str]) -> TpResult<()> {
        self.binding_checks += 1;
        if !self.allocations.is_empty()
            || self.image != Some(image)
            || kernels != crate::tp_artifact::ENGINEERING_TP_LARGE_KV_EXPORTS_V9
        {
            return Err("recorded image mismatch".into());
        }
        Ok(())
    }
    fn peer_group_rank(&self) -> Option<(u32, u32, u32)> {
        self.peer.then_some((1, 0, 1))
    }
    fn allocate(&mut self, bytes: usize) -> TpResult<u64> {
        self.allocations.push(bytes);
        Ok(u64::try_from(self.allocations.len()).unwrap())
    }
    fn write(&mut self, _: u64, _: usize, _: &[u8]) -> TpResult<()> {
        Err("no device payload".into())
    }
    fn read(&mut self, _: u64, _: usize, _: &mut [u8]) -> TpResult<()> {
        Err("no device payload".into())
    }
    fn submit(&mut self, _: &EngineeringTpDispatchV1) -> TpResult<()> {
        Err("no device execution".into())
    }
    fn wait(&mut self) -> TpResult<()> {
        Err("no device execution".into())
    }
    fn close(&mut self) -> TpResult<()> {
        Ok(())
    }
}

fn large_pool(pages: u32, context: u32) -> EngineeringTpPagedPoolV1 {
    EngineeringTpPagedPoolV1::new_large_kv32_recording(
        pool().scope(),
        EngineeringTpPagedLimitsV1::new_large_kv32(context, 32, pages, 100).unwrap(),
    )
    .unwrap()
}

#[test]
fn admitted_image_is_required_before_allocation_even_for_small_large_profile() {
    for pages in [4, 512, 513, 16384] {
        let pool = large_pool(pages, 8192);
        let mut transports = [Sparse::default()];
        assert!(validate_pool_binding(&mut transports, target(), &pool, 32, true).is_err());
        assert_eq!(transports[0].binding_checks, 1);
        assert!(transports[0].allocations.is_empty());
        transports[0].image = Some([18; 32]);
        assert!(validate_pool_binding(&mut transports, target(), &pool, 32, true).is_err());
        transports[0].image = Some([19; 32]);
        assert!(validate_pool_binding(&mut transports, target(), &pool, 32, false).is_err());
        assert!(validate_pool_binding(&mut transports, target(), &pool, 16, true).is_err());
        validate_pool_binding(&mut transports, target(), &pool, 32, true).unwrap();
        transports[0].peer = true;
        assert!(validate_pool_binding(&mut transports, target(), &pool, 32, true).is_err());
        assert!(transports[0].allocations.is_empty());
    }
    let mut pool = large_pool(4, 64);
    pool.open_sequence(pool.scope(), &[1], 0).unwrap();
    let mut transports = [Sparse {
        image: Some([19; 32]),
        ..Sparse::default()
    }];
    assert!(validate_pool_binding(&mut transports, target(), &pool, 32, true).is_err());
    assert_eq!(transports[0].binding_checks, 0);
    assert!(
        validate_pool_binding::<Sparse>(&mut [], target(), &large_pool(4, 64), 32, true).is_err()
    );
    let mut ranks = [Sparse::default(), Sparse::default()];
    assert!(validate_pool_binding(&mut ranks, target(), &large_pool(4, 64), 32, true).is_err());
}

#[test]
fn full_storage_has_exact_36_gib_payload_but_logical_capacity_remains_8192() {
    let pool = large_pool(16384, 8192);
    let mut transport = Sparse {
        image: Some([19; 32]),
        ..Sparse::default()
    };
    validate_pool_binding(
        std::slice::from_mut(&mut transport),
        target(),
        &pool,
        32,
        true,
    )
    .unwrap();
    let plan = Qwen3TensorParallelPlanV1::new(target(), 1).unwrap();
    let rank = allocate_rank_storage(
        &mut transport,
        &plan,
        0,
        pool.limits().physical_token_capacity().unwrap(),
        32,
    )
    .unwrap();
    let mut total = 0_u64;
    for layer in &rank.layers {
        for tensor in [layer.k_cache, layer.v_cache] {
            assert_eq!(tensor.elements, 268_435_456);
            assert_eq!(tensor.element_bytes, 2);
            assert_eq!(
                transport.allocations[usize::try_from(tensor.id - 1).unwrap()],
                536_870_912
            );
            total += u64::try_from(tensor.elements).unwrap() * 2;
        }
    }
    assert_eq!(total, 38_654_705_664);
    assert_eq!(total, pool.limits().target_kv_payload_bytes().unwrap());
    assert!(TensorParallelSequenceV1::new(pool.limits().context_tokens(), 151_936).is_ok());
    assert!(
        TensorParallelSequenceV1::new(pool.limits().physical_token_capacity().unwrap(), 151_936)
            .is_err()
    );
}

#[test]
fn explicit_large_pool_routes_only_two_roots_and_rejects_wave_and_sequences() {
    for rows in [1, 16, 17, 32] {
        let mut pool = large_pool(4, 64);
        let mut driver = fixture(1, &pool);
        assert!(driver.configure_wave_attention(true).is_err());
        assert!(driver.configure_dispatch_sequences(true).is_err());
        assert!(
            driver
                .configure_reduction(EngineeringTpReductionModeV3::DevicePeerV4)
                .is_err()
        );
        driver.configure_head_precision_v8(true).unwrap();
        let prompt = vec![1; rows];
        let sequence = pool
            .open_sequence(pool.scope(), &prompt, 0)
            .unwrap()
            .sequence();
        let batch = pool
            .reserve_batch(
                &(0..rows)
                    .map(|position| EngineeringTpPageRowV1 {
                        sequence,
                        token: 1,
                        position: u32::try_from(position).unwrap(),
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap();
        pool.begin_submission(&batch).unwrap();
        let output = driver.execute(&batch).unwrap();
        assert_eq!(output.choices.len(), rows);
        assert_eq!(driver.dispatch_counts(), [544]);
        let commands = &driver.inner.transports[0].commands;
        for name in crate::tp_artifact::ENGINEERING_TP_LARGE_KV_EXPORTS_V9 {
            let selected = commands
                .iter()
                .filter(|command| command.kernel == name)
                .collect::<Vec<_>>();
            assert_eq!(selected.len(), 36);
            assert_eq!(
                selected[0].grid_workgroups,
                if name.contains("append") {
                    1
                } else {
                    u32::try_from(rows).unwrap() * 32
                }
            );
        }
        assert!(
            !commands
                .iter()
                .any(|command| command.kernel.contains("batch32_paged_"))
        );
        pool.commit_batch(&batch, output.completion).unwrap();
        driver.close().unwrap();
    }
}
