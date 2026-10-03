use super::*;

fn collect(source: &[u8], rows: usize, columns: usize, cap: usize) -> Vec<u8> {
    let mut output = Vec::new();
    stream_transpose(source, rows, columns, cap, &mut |offset, bytes| {
        assert_eq!(offset, output.len());
        assert!(bytes.len() <= cap);
        output.extend_from_slice(bytes);
        Ok(())
    })
    .unwrap();
    output
}

#[test]
fn transpose_preserves_all_bf16_bits_and_destination_order() {
    let words = [
        0u16, 0x8000, 0x7f80, 0xff80, 0x7fc1, 0x7f81, 0xffff, 1, 0x3f80, 0x1234, 0xabcd, 0x7fff,
        0x0080, 0x007f, 0xdead,
    ];
    let bytes: Vec<_> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
    let expected: Vec<_> = (0..5)
        .flat_map(|column| (0..3).flat_map(move |row| words[row * 5 + column].to_le_bytes()))
        .collect();
    for cap in [6, 12, 18, 30, MAX_CHUNK] {
        let actual = collect(&bytes, 3, 5, cap);
        assert_eq!(actual, expected);
        assert_eq!(Sha256::digest(&actual), Sha256::digest(&expected));
    }
}

#[test]
fn transpose_roundtrip_uses_opposite_dimensions() {
    let source: Vec<_> = (0..77u16).flat_map(|n| (n * 631).to_le_bytes()).collect();
    let transposed = collect(&source, 7, 11, 42);
    assert_eq!(collect(&transposed, 11, 7, 44), source);
}

#[test]
fn transpose_validates_every_bound_before_first_sink() {
    for (rows, columns, len, cap) in [
        (0, 2, 0, 4),
        (2, 0, 0, 4),
        (2, 2, 7, 4),
        (2, 2, 8, 2),
        (2, 2, 8, 5),
        (2, 2, 8, MAX_CHUNK + 2),
        (usize::MAX, 2, 0, 4),
    ] {
        let mut calls = 0;
        assert!(
            stream_transpose(&vec![0; len], rows, columns, cap, &mut |_, _| {
                calls += 1;
                Ok(())
            })
            .is_err()
        );
        assert_eq!(calls, 0);
    }
}

#[test]
fn transpose_sink_failure_never_retries_or_emits_later_chunks() {
    let mut calls = 0;
    let result = stream_transpose(&[0; 30], 3, 5, 6, &mut |offset, _| {
        calls += 1;
        if offset == 6 {
            Err("injected sink failure".into())
        } else {
            Ok(())
        }
    });
    assert_eq!(result.unwrap_err(), "injected sink failure");
    assert_eq!(calls, 2);
}

#[test]
fn head_geometry_is_distinct_k_by_n_and_has_bounded_rows() {
    assert_eq!(BYTES, 1_244_659_712);
    assert_eq!(MAX_CHUNK / (N * 2), 13);
    assert_eq!(13 * N * 2, 3_950_336);
    assert_eq!(K % 13, 1);
}
