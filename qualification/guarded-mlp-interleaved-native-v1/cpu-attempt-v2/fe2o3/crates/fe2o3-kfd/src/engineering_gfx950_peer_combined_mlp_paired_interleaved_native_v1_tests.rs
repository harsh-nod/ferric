//! Four retained pairs: two layers in each bank, revisited after intervening work.
use super::*;

const SCALES: [i32; 8] = [-1, 1, -2, 2, -4, 4, -8, 8];
const WRITABLE: [usize; 6] = [0, 5, 6, 7, 8, 9];
const FINAL_HASHES: [[&str; 2]; 8] = [
    [
        "b20e302f292f34269bcb8050df66f0a0f3c5e05cd784f5aef70f049473422f73",
        "9bbbfec933d22db9ed77aa6fb3af82cf7bbe5fb51f0f2d58066fed2c91dbaad4",
    ],
    [
        "6e6ecb4b75e6e5f09a212bbd4d4779fc2975c026f4398c2241230843cf4a33bd",
        "9fde0e89ff2a46d8b366a4adc5dbe4bb1728106112b2b8fa563e4af6d944d3bd",
    ],
    [
        "dc168a4cf10046bccf128222d385ad604427d5527fd61477c8ab65aaf38b0157",
        "d67e525ebc822f08791ee7d44f34ca4e5d39a8bf8e93114bbafa0b53f54dac02",
    ],
    [
        "403903da99ade382e932312fb9fcdaa5d5fe3e41bf05768f614667fd77b160bf",
        "e7164a702f121bbaa1b9e940960177f2b7670bf279f641e6e827be6bdfba3e93",
    ],
    [
        "0403c9a2c2f7eb3015fa56f8c8193c2ad3fec071adff624703c7ae06ec4aea33",
        "c80e80be3a90c3cf3e0067983de7023be3407c84f652e25ee32d3fa31eaa68a6",
    ],
    [
        "1ce5c1c95bacc944f878faadb899b26b4b44ac0a9db0599af82e8681d456edcb",
        "b197b000825190db35241e80fb84462eb64b938fc3939f1298b2daab47dc41f5",
    ],
    [
        "e4abf09c2e64dcd0a4dad294a64f44da0bd124085f0ff5bc8fb7a3fabfb54b88",
        "45c1625266516424ffaef254271d0dfd07b41ea3966030b714af47554414dc49",
    ],
    [
        "62d3a5411e727f0e514798c9992de1a9bcaadb627751925cb1deba6ea9bc9fca",
        "9ba51ea44d9a0d5d4cfde87b36d943191730e0e2832993986766f37bc11c0eef",
    ],
];
const UP_HASHES: [[&str; 2]; 8] = [
    [
        "3fced08dc5eb0444bad194ea4f5d12fc3b1a92682d661fa66da3f9f778ea8c72",
        "97191553d52fdd9bdd6e770ecc4374d1ae41a16f96fb87a710d13b6494d1374f",
    ],
    [
        "e8acc9025230b0b6e92839eeaa86f437a6400f7854796e6ee6ed4b87ef9d1cc7",
        "2086136b0338d4a163c317960b009767f7f7cf4ce175062011c54dfa2dd41f52",
    ],
    [
        "9171b5727cd208fb2a244636d107eff7a48a0a0cb8fe9f3b0cd43ad66620cc0f",
        "8786800f14eb32fb807803cd264aebe628e677941902aa14ef0dcbfb48c24ad7",
    ],
    [
        "4d8d301047aab087afdf5c54302051fd4848fac603ac85605688d3b1e1a47da5",
        "1c1d077694c7f6a826ff26c6a770ee9887cb9c376d6f65193545df3180583dd9",
    ],
    [
        "a4533c4ab7ca44f99e41411a84fad4515d9e124006c3c0c4941e6db4e7cfbe7e",
        "65d5fd51892e13e7ca24f3263f239f2a4ebbb53a3fdef3f2b89265cd71430c22",
    ],
    [
        "4d0c73076df0025f92118bcabd2180d3e22857c7deb57c596bd53d669747a8bb",
        "8b5fe43a4d6c5b37a5f57edf4997722ea0905e4d195a39f316f16070e5a13496",
    ],
    [
        "d74849799bdb08d8a97bb9b5983bc0ce0184890f768495a85729c7d9f2726da3",
        "2db79e5ca535928334b679432fb512b40e5204b1d54395c9aad7bb4de8be9c58",
    ],
    [
        "f739f4a4b7e631cec45ee4d06517abe659b0794f78f872d8b5748c9c09fd240c",
        "69555e7c6621a33cf87206a21051e9af52c4e716beda891b93561ac92ce387f5",
    ],
];

fn schedule(forward: usize, layer: usize) -> Result<(usize, u64, i32)> {
    if forward >= 4 || layer >= 2 {
        return Err("interleaved finite schedule".into());
    }
    Ok((
        forward % 2,
        (forward / 2 + 1) as u64,
        SCALES[2 * forward + layer],
    ))
}

fn scaled_root(rank: usize, index: usize, scale: i32) -> Result<Vec<u8>> {
    if rank >= 2 || !SCALES.contains(&scale) {
        return Err("interleaved rank/scale".into());
    }
    match index {
        3 => {
            let mut bytes = vec![0; profile::EXTENTS[3]];
            // Preserve canonical +0 padding for negative scales too.
            for row in 0..INNER {
                let col = (31 * row + 43 * rank + 11) % WIDTH;
                let at = 2 * (row * WIDTH + col);
                bytes[at..at + 2].copy_from_slice(
                    &narrow(scale as f32 * up_value(rank, row) * sign(rank, col))?.to_le_bytes(),
                );
            }
            Ok(bytes)
        }
        7 => bf16_values(INNER, |i| scale as f32 * up_value(rank, i)),
        8 => bf16_values(INNER, |i| {
            scale as f32 * gate_value(rank, i) * up_value(rank, i)
        }),
        9 => Ok(f32_values(WIDTH, |i| scale as f32 * down_value(rank, i))),
        _ => expected_root(rank, index),
    }
}

fn scaled_final(rank: usize, scale: i32) -> Result<Vec<u8>> {
    if rank >= 2 || !SCALES.contains(&scale) {
        return Err("interleaved final rank/scale".into());
    }
    (0..WIDTH)
        .map(|i| {
            reference(
                scale as f32 * down_value(0, i),
                scale as f32 * down_value(1, i),
                narrow(sign(rank, i))?,
            )
        })
        .collect::<Result<Vec<_>>>()
        .map(|words| words.into_iter().flat_map(u16::to_le_bytes).collect())
}

struct Payloads {
    roots: [[Gfx950EngineeringPeerBufferV1; 10]; 2],
    outputs: [Gfx950EngineeringPeerBufferV1; 2],
}

fn inputs<'a>(
    kernels: &'a [[Gfx950EngineeringPeerKernelV1; 4]],
    payloads: &Payloads,
    partials: [Gfx950EngineeringPeerBufferV1; 2],
    residuals: [Gfx950EngineeringPeerBufferV1; 2],
    hashes: [[u8; 32]; 3],
) -> coordinator::Inputs<'a> {
    coordinator::Inputs {
        ranks: std::array::from_fn(|rank| coordinator::RankInputs {
            kernels: kernels[rank].each_ref(),
            mlp_roots: payloads.roots[rank],
            residual_input: residuals[rank],
            output: payloads.outputs[rank],
        }),
        partials,
        projection_sha256: hashes[0],
        mlp_sha256: hashes[1],
    }
}

fn observe_pairs(
    group: &mut Gfx950EngineeringPeerGroupV1,
    pairs: &mut [coordinator::RetainedPair],
    identities: &mut Option<[[[u64; 4]; 2]; 4]>,
    write: u64,
    timeout: u32,
) -> Result<serde_json::Value> {
    if pairs.len() != 4 {
        return Err("interleaved pair census".into());
    }
    let mut records = Vec::new();
    let mut ids = [[[0; 4]; 2]; 4];
    let mut distinct = BTreeSet::new();
    for (index, pair) in pairs.iter_mut().enumerate() {
        let observed = pair.observe_for_test(group, timeout)?;
        ids[index] = observed.owners;
        for rank in 0..2 {
            if observed.owners[rank][0] != group.incarnation
                || observed.owners[rank][2..] != [rank as u64, 2208]
                || !distinct.insert(observed.owners[rank][1])
                || observed.frontiers[rank].0 != write
            {
                return Err("interleaved owner identity or queue frontier".into());
            }
        }
        let prefix_sha: [String; 2] = observed.states.each_ref().map(|s| {
            digest(
                &s.prefix
                    .iter()
                    .flat_map(|v| v.to_le_bytes())
                    .collect::<Vec<_>>(),
            )
        });
        records.push(serde_json::json!({"pair":index,"owners":observed.owners,
            "generation":observed.generation,"completed":observed.completed,"prefix_sha256":prefix_sha,
            "guards":observed.states.each_ref().map(|s|s.guard),"observed_queue_frontiers":observed.frontiers}));
    }
    if let Some(expected) = identities {
        if *expected != ids {
            return Err("interleaved owner storage changed".into());
        }
    } else {
        *identities = Some(ids);
    }
    Ok(serde_json::Value::Array(records))
}

fn check_inactive(
    group: &mut Gfx950EngineeringPeerGroupV1,
    payloads: &[Payloads],
    active: usize,
    last_scale: &[Option<i32>; 4],
) -> Result<serde_json::Value> {
    let mut records = Vec::new();
    for (pair, p) in payloads.iter().enumerate() {
        if pair == active {
            continue;
        }
        for rank in 0..2 {
            let mut roots = Vec::new();
            for index in WRITABLE {
                let expected = match last_scale[pair] {
                    Some(scale) => scaled_root(rank, index, scale)?,
                    None => poison(index),
                };
                let sha = check_all(group, p.roots[rank][index], &expected)?;
                roots.push(serde_json::json!({"root":index,"sha256":sha}));
            }
            let expected = match last_scale[pair] {
                Some(scale) => scaled_final(rank, scale)?,
                None => poison(0),
            };
            let output_sha = check_all(group, p.outputs[rank], &expected)?;
            records.push(serde_json::json!({"pair":pair,"rank":rank,"scale":last_scale[pair],"roots":roots,"output_sha256":output_sha}));
        }
    }
    Ok(serde_json::Value::Array(records))
}

fn interleaved_exercise(
    group: &mut Gfx950EngineeringPeerGroupV1,
    req: &PairedRequest,
    images: &[Vec<u8>; 3],
    hashes: [[u8; 32]; 3],
) -> Result<serde_json::Value> {
    let mut kernels = Vec::<[Gfx950EngineeringPeerKernelV1; 4]>::new();
    let mut common = Vec::<[Gfx950EngineeringPeerBufferV1; 4]>::new();
    for rank in 0..2 {
        let mut row = Vec::new();
        for (image, symbol) in [
            (0, R1_SYMBOL),
            (1, profile::SYMBOL),
            (2, GUARD_SYMBOL),
            (2, R2_SYMBOL),
        ] {
            row.push(group.load_kernel(
                rank,
                images[image].clone(),
                hashes[image],
                symbol.into(),
            )?);
        }
        kernels.push(row.try_into().map_err(|_| "interleaved kernel count")?);
        let mut row = Vec::new();
        for index in 1..=4 {
            let token = group.allocate(rank, &[], profile::EXTENTS[index] as u64)?;
            write_all(group, token, &expected_root(rank, index)?)?;
            row.push(token);
        }
        common.push(
            row.try_into()
                .map_err(|_| "interleaved shared root count")?,
        );
    }
    let partials = [
        group.allocate(0, &[1], 16384)?,
        group.allocate(1, &[0], 16384)?,
    ];
    let residuals = [group.allocate(0, &[], 8192)?, group.allocate(1, &[], 8192)?];
    for rank in 0..2 {
        write_all(group, partials[rank], &original_partial(rank))?;
        write_all(group, residuals[rank], &original_residual(rank)?)?;
    }
    let mut payloads = Vec::new();
    let mut pairs = Vec::new();
    for pair_index in 0..4 {
        let mut roots = Vec::<[Gfx950EngineeringPeerBufferV1; 10]>::new();
        let mut outputs = Vec::new();
        for rank in 0..2 {
            let mut row = Vec::new();
            for index in 0..10 {
                let token = if (1..=4).contains(&index) {
                    common[rank][index - 1]
                } else {
                    let peers = if index == 9 { vec![1 - rank] } else { vec![] };
                    let token = group.allocate(rank, &peers, profile::EXTENTS[index] as u64)?;
                    write_all(group, token, &poison(index))?;
                    token
                };
                row.push(token);
            }
            roots.push(
                row.try_into()
                    .map_err(|_| "interleaved private root count")?,
            );
            let output = group.allocate(rank, &[], 8192)?;
            write_all(group, output, &poison(0))?;
            outputs.push(output);
        }
        let p = Payloads {
            roots: roots.try_into().map_err(|_| "interleaved rank count")?,
            outputs: outputs.try_into().map_err(|_| "interleaved output count")?,
        };
        // SAFETY: reviewed exact images and role bindings, separate writable roots,
        // completed coherent host writes, and one dedicated disposable process.
        // Shared Up is rewritten only between synchronous completed segments.
        pairs.push(unsafe {
            coordinator::RetainedPair::allocate(
                group,
                &inputs(&kernels, &p, partials, residuals, hashes),
                req.timeout_ms,
            )
            .map_err(|error| format!("interleaved allocate pair {pair_index}: {error}"))?
        });
        payloads.push(p);
    }
    let mut identities = None;
    let initial = observe_pairs(group, &mut pairs, &mut identities, 0, req.timeout_ms)?;
    let mut last_scale = [None; 4];
    let mut events = Vec::new();
    let mut write = 0;
    for forward in 0..4 {
        let (bank, generation, _) = schedule(forward, 0)?;
        if forward >= 2 {
            let before = observe_pairs(group, &mut pairs, &mut identities, write, req.timeout_ms)?;
            let next =
                coordinator::rearm_pairs(group, &mut pairs[2 * bank..2 * bank + 2], req.timeout_ms)
                    .map_err(|error| {
                        format!("interleaved rearm forward {forward} bank {bank}: {error}")
                    })?;
            if next != generation {
                return Err("interleaved local generation".into());
            }
            let after = observe_pairs(group, &mut pairs, &mut identities, write, req.timeout_ms)?;
            events.push(serde_json::json!({"kind":"rearm","forward":forward,"bank":bank,"next_generation":next,"before":before,"after":after}));
        }
        for layer in 0..2 {
            let (_, _, scale) = schedule(forward, layer)?;
            let index = 2 * bank + layer;
            let p = &payloads[index];
            for rank in 0..2 {
                write_all(group, p.roots[rank][3], &scaled_root(rank, 3, scale)?)?;
                for root in WRITABLE {
                    write_all(group, p.roots[rank][root], &poison(root))?;
                }
                write_all(group, p.outputs[rank], &poison(0))?;
            }
            let completed = pairs[index]
                .run(
                    group,
                    inputs(&kernels, p, partials, residuals, hashes),
                    req.timeout_ms,
                )
                .map_err(|error| {
                    format!("interleaved run forward {forward} bank {bank} layer {layer}: {error}")
                })?;
            write += 5;
            let mut records = Vec::new();
            for rank in 0..2 {
                require_terminal(&completed.states[rank], generation)?;
                let mut stages = Vec::new();
                for root in 0..10 {
                    let expected = scaled_root(rank, root, scale)?;
                    let sha = check_all(group, p.roots[rank][root], &expected)?;
                    let actual = if matches!(root, 2..=4) {
                        None
                    } else {
                        Some(hex(&group.read(
                            p.roots[rank][root],
                            0,
                            expected.len() as u32,
                        )?))
                    };
                    stages.push(serde_json::json!({"root":root,"bytes":expected.len(),"sha256":sha,"le_hex":actual}));
                }
                let output_sha = check_all(group, p.outputs[rank], &scaled_final(rank, scale)?)?;
                let partial_sha = check_all(group, partials[rank], &original_partial(rank))?;
                let residual_sha = check_all(group, residuals[rank], &original_residual(rank)?)?;
                records.push(serde_json::json!({"rank":rank,"stages":stages,
                    "partial_le_hex":hex(&group.read(partials[rank],0,16384)?),"partial_sha256":partial_sha,
                    "residual_le_hex":hex(&group.read(residuals[rank],0,8192)?),"residual_sha256":residual_sha,
                    "output_le_hex":hex(&group.read(p.outputs[rank],0,8192)?),"output_sha256":output_sha,
                    "prefix":completed.states[rank].prefix.to_vec(),"guard":completed.states[rank].guard}));
            }
            last_scale[index] = Some(scale);
            let inactive = check_inactive(group, &payloads, index, &last_scale)?;
            let owners = observe_pairs(group, &mut pairs, &mut identities, write, req.timeout_ms)?;
            events.push(serde_json::json!({"kind":"segment","forward":forward,"bank":bank,"layer":layer,
                "generation":generation,"scale":scale,"records":records,"inactive_payloads":inactive,"owners":owners,
                "observed_queue_frontiers":completed.observed_queue_frontiers,"segment_host_ns":completed.segment_host_ns}));
        }
    }
    Ok(
        serde_json::json!({"initial":initial,"events":events,"kernel_dispatches":64,"barrier_packets":16,
        "completion_signals":80,"paired_coordinator_tested":true,"owner_lifecycle_tested":true,"rearm_tested":true,
        "owners_reused":true,"payloads_reused":true,"fresh_arenas_per_segment":true,"interleaving_tested":true,
        "inactive_payload_checks":336,"fixture":"two-layer-two-bank-signed-up-v1","performance_claim":false,"canaries_tested":false}),
    )
}

#[test]
#[ignore = "requires explicit MI350 two-layer two-bank interleaving and three checked images"]
fn native_paired_guarded_mlp_interleaved_v1() {
    let result = paired_run_with(
        interleaved_exercise,
        "ferric-native-paired-guarded-mlp-interleaved-observation-v1",
    )
    .expect("native interleaved guarded MLP failed");
    println!("FERRIC_NATIVE_PAIRED_INTERLEAVED_MLP_V1={result}");
}

#[test]
fn interleaved_schedule_keeps_bank_generations_separate_from_forwards() {
    for forward in 0..4 {
        for layer in 0..2 {
            assert_eq!(
                schedule(forward, layer).unwrap(),
                (
                    forward % 2,
                    [1, 1, 2, 2][forward],
                    SCALES[2 * forward + layer]
                )
            );
        }
    }
    assert!(schedule(4, 0).is_err());
    assert!(schedule(0, 2).is_err());
    assert!(scaled_final(0, 0).is_err());
    assert!(scaled_root(2, 3, 1).is_err());
}

#[test]
fn interleaved_all_eight_final_witnesses_match_independent_hashes() {
    let zeros = [
        [0, 0],
        [0, 0],
        [1228, 1229],
        [820, 819],
        [0, 0],
        [0, 0],
        [272, 273],
        [411, 410],
    ];
    for rank in 0..2 {
        let mut prior: Vec<Vec<u8>> = Vec::new();
        for (index, scale) in SCALES.into_iter().enumerate() {
            let actual = scaled_final(rank, scale).unwrap();
            assert_eq!(digest(&actual), FINAL_HASHES[index][rank]);
            assert_eq!(
                actual.chunks_exact(2).filter(|b| *b == [0, 0]).count(),
                zeros[index][rank]
            );
            assert!(actual.chunks_exact(2).all(|b| b != [0, 128]));
            for old in &prior {
                assert!(
                    actual
                        .chunks_exact(2)
                        .zip(old.chunks_exact(2))
                        .all(|(a, b)| a != b)
                );
            }
            prior.push(actual);
        }
    }
}

#[test]
fn interleaved_signed_up_preserves_positive_zero_dense_padding() {
    for rank in 0..2 {
        for (index, scale) in SCALES.into_iter().enumerate() {
            let body = scaled_root(rank, 3, scale).unwrap();
            assert_eq!(digest(&body), UP_HASHES[index][rank]);
            assert_eq!(body.len(), 50_331_648);
            assert_eq!(
                body.chunks_exact(2).filter(|word| *word != [0, 0]).count(),
                INNER
            );
            assert!(body.chunks_exact(2).all(|word| word != [0, 128]));
        }
    }
}
