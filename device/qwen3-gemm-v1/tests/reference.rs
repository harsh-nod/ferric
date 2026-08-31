use fe2o3_device::Bf16;

fn oracle_bf16_from_f32(value: f32) -> u16 {
    let bits = value.to_bits();
    if (bits & 0x7f80_0000) == 0x7f80_0000 && (bits & 0x007f_ffff) != 0 {
        return ((bits >> 16) as u16) | 0x0040;
    }

    let retained_lsb = (bits >> 16) & 1;
    ((bits + 0x7fff + retained_lsb) >> 16) as u16
}

fn oracle_bf16_to_f32(bits: u16) -> f32 {
    f32::from_bits(u32::from(bits) << 16)
}

fn oracle_gemm_reference(
    a: &[u16],
    b: &[u16],
    prior_c: &[u16],
    m: usize,
    n: usize,
    k: usize,
    beta_one: bool,
) -> Vec<u16> {
    assert_eq!(a.len(), m * k);
    assert_eq!(b.len(), k * n);
    assert_eq!(prior_c.len(), m * n);

    let mut output = Vec::with_capacity(m * n);
    for row in 0..m {
        for column in 0..n {
            let mut accumulator = 0.0_f32;
            for reduction in 0..k {
                let left = oracle_bf16_to_f32(a[row * k + reduction]);
                let right = oracle_bf16_to_f32(b[reduction * n + column]);
                let product = left * right;
                accumulator = accumulator + product;
            }
            if beta_one {
                accumulator = accumulator + oracle_bf16_to_f32(prior_c[row * n + column]);
            }
            output.push(oracle_bf16_from_f32(accumulator));
        }
    }
    output
}

fn oracle_gemm_a4(
    a: &[u16],
    b: &[u16],
    prior_c: &[u16],
    m: usize,
    n: usize,
    k: usize,
    beta_one: bool,
) -> Vec<u16> {
    assert_eq!(a.len(), m * k);
    assert_eq!(b.len(), k * n);
    assert_eq!(prior_c.len(), m * n);
    assert_eq!(k % 4, 0);

    let mut output = Vec::with_capacity(m * n);
    for row in 0..m {
        for column in 0..n {
            let mut accumulator = 0.0_f32;
            for reduction in (0..k).step_by(4) {
                for offset in 0..4 {
                    let left = oracle_bf16_to_f32(a[row * k + reduction + offset]);
                    let right = oracle_bf16_to_f32(b[(reduction + offset) * n + column]);
                    let product = left * right;
                    accumulator = accumulator + product;
                }
            }
            if beta_one {
                accumulator = accumulator + oracle_bf16_to_f32(prior_c[row * n + column]);
            }
            output.push(oracle_bf16_from_f32(accumulator));
        }
    }
    output
}

fn bf16_values(values: &[f32]) -> Vec<u16> {
    values.iter().copied().map(oracle_bf16_from_f32).collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EmbeddingError {
    InvalidToken { row: usize, token: u32 },
}

fn oracle_embedding_copy(
    tokens: &[u32],
    weight: &[u16],
    hidden: usize,
    vocabulary: u32,
) -> Result<Vec<u16>, EmbeddingError> {
    assert_eq!(weight.len(), vocabulary as usize * hidden);
    let mut output = Vec::with_capacity(tokens.len() * hidden);
    for (row, token) in tokens.iter().copied().enumerate() {
        if token >= vocabulary {
            return Err(EmbeddingError::InvalidToken { row, token });
        }
        let start = token as usize * hidden;
        output.extend_from_slice(&weight[start..start + hidden]);
    }
    Ok(output)
}

#[test]
fn independent_bf16_oracle_matches_exact_rne_and_special_value_policy() {
    let cases = [
        (0x0000_0000, 0x0000),
        (0x8000_0000, 0x8000),
        (0x3f80_0000, 0x3f80),
        (0x3f80_8000, 0x3f80),
        (0x3f81_8000, 0x3f82),
        (0x7f80_0000, 0x7f80),
        (0xff80_0000, 0xff80),
        (0x7f80_0001, 0x7fc0),
        (0xff80_0001, 0xffc0),
        (0x7fa1_0001, 0x7fe1),
    ];

    for (input_bits, expected) in cases {
        let value = f32::from_bits(input_bits);
        assert_eq!(oracle_bf16_from_f32(value), expected);
        assert_eq!(Bf16::from_f32(value).to_bits(), expected);
        assert_eq!(
            Bf16::from_bits(expected).to_f32().to_bits(),
            u32::from(expected) << 16
        );
    }
}

#[test]
fn reference_oracle_has_known_ascending_fp32_and_residual_results() {
    let a = bf16_values(&[1.0, 2.0, 3.0, 4.0, -1.0, 0.5, 2.0, -2.0]);
    let b = bf16_values(&[
        1.0, 0.0, -1.0, 0.5, 2.0, 1.0, -1.0, 1.0, 0.25, 2.0, -0.5, 0.0,
    ]);
    let prior_c = vec![oracle_bf16_from_f32(1.0); 6];

    assert_eq!(
        oracle_gemm_reference(&a, &b, &prior_c, 2, 3, 4, false),
        vec![0x40e0, 0x40a0, 0x3fe0, 0xc0d8, 0x4080, 0x4000]
    );
    assert_eq!(
        oracle_gemm_reference(&a, &b, &prior_c, 2, 3, 4, true),
        vec![0x4100, 0x40c0, 0x4030, 0xc0b8, 0x40a0, 0x4040]
    );
}

#[test]
fn reference_and_a4_oracles_match_on_rounding_sensitive_tail_dimensions() {
    let m = 3;
    let n = 5;
    let k = 8;
    let palette = [
        -3.5_f32,
        -1.25,
        -0.5,
        -0.011_718_75,
        0.0,
        0.003_906_25,
        0.75,
        1.0,
        1.5,
        2.25,
    ];
    let a = (0..m * k)
        .map(|index| oracle_bf16_from_f32(palette[(index * 7 + 3) % palette.len()]))
        .collect::<Vec<_>>();
    let b = (0..k * n)
        .map(|index| oracle_bf16_from_f32(palette[(index * 3 + 1) % palette.len()]))
        .collect::<Vec<_>>();
    let prior_c = (0..m * n)
        .map(|index| oracle_bf16_from_f32(palette[(index * 9 + 2) % palette.len()]))
        .collect::<Vec<_>>();

    for beta_one in [false, true] {
        assert_eq!(
            oracle_gemm_a4(&a, &b, &prior_c, m, n, k, beta_one),
            oracle_gemm_reference(&a, &b, &prior_c, m, n, k, beta_one)
        );
    }
}

#[test]
fn beta_zero_ignores_prior_nan_and_beta_one_rounds_ties_to_even() {
    let a = bf16_values(&[1.0, 0.0, 0.0, 0.0]);
    let b = bf16_values(&[1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);

    let nan_prior = [0x7fc1, 0xffc2];
    assert_eq!(
        oracle_gemm_a4(&a, &b, &nan_prior, 1, 2, 4, false),
        vec![0x3f80, 0x3f80]
    );

    let tie_prior = [0x3b80, 0x3c40];
    assert_eq!(
        oracle_gemm_a4(&a, &b, &tie_prior, 1, 2, 4, true),
        vec![0x3f80, 0x3f82]
    );
}

#[test]
fn embedding_oracle_copies_bits_exactly_and_rejects_oob_tokens() {
    let weight = [
        0x0000, 0x8000, 0x7f80, 0x7fc1, 0x3f80, 0xbf80, 0x0001, 0x7f7f, 0x4040, 0xc040, 0x3b80,
        0xffff,
    ];
    assert_eq!(
        oracle_embedding_copy(&[2, 0, 1], &weight, 4, 3),
        Ok(vec![
            0x4040, 0xc040, 0x3b80, 0xffff, 0x0000, 0x8000, 0x7f80, 0x7fc1, 0x3f80, 0xbf80, 0x0001,
            0x7f7f,
        ])
    );
    assert_eq!(
        oracle_embedding_copy(&[1, 3], &weight, 4, 3),
        Err(EmbeddingError::InvalidToken { row: 1, token: 3 })
    );
}
