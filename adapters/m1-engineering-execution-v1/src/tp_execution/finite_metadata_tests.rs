use super::super::rope_bytes;
use super::*;
use std::collections::BTreeSet;

fn input(position: u32) -> EngineeringTp2GraphInputV1 {
    let active = usize::try_from(position / 16 + 1).unwrap();
    let mut page_table = vec![u32::MAX; PAGE_COUNT];
    for (logical, page) in page_table.iter_mut().take(active).enumerate() {
        *page = u32::try_from(PAGE_COUNT - 1 - logical).unwrap();
    }
    let (mut cos_sin, sin) = rope_bytes(position, 1_000_000);
    cos_sin.extend(sin);
    EngineeringTp2GraphInputV1 {
        geometry: EngineeringTp2GraphGeometryV1::Long2304,
        plan_sha256: [3; 32],
        generation: u64::from(position) + 1,
        epoch: u64::from(position),
        token: 785,
        position,
        page_table,
        cos_sin,
    }
}

fn prepare(input: &EngineeringTp2GraphInputV1) -> EngineeringTp2Finite2304MetadataV1 {
    EngineeringTp2Finite2304MetadataV1::prepare(input).unwrap()
}

#[test]
fn finite_metadata_boundary_positions_preserve_every_active_page() {
    for position in [0, 1, 15, 16, 2047, 2303] {
        let input = input(position);
        let before = input.clone();
        let metadata = prepare(&input);
        let active = usize::try_from(position / 16 + 1).unwrap();
        assert_eq!(metadata.cache_metadata()[0], position);
        assert_eq!(
            &metadata.cache_metadata()[1..=active],
            &input.page_table[..active]
        );
        let actual: BTreeSet<_> = metadata.cache_metadata()[1..].iter().copied().collect();
        let expected: BTreeSet<_> = (0..144).collect();
        assert_eq!(actual, expected);
        let tail: Vec<_> = (0..u32::try_from(PAGE_COUNT - active).unwrap()).collect();
        assert_eq!(&metadata.cache_metadata()[active + 1..], tail);
        assert_eq!(input, before);
    }
}

#[test]
fn finite_metadata_tail_is_deterministic_for_nonidentity_active_pages() {
    let mut input = input(32);
    input.page_table[..3].copy_from_slice(&[19, 2, 142]);
    let first = prepare(&input);
    let second = prepare(&input);
    assert_eq!(first.cache_metadata(), second.cache_metadata());
    assert_eq!(&first.cache_metadata()[1..4], &[19, 2, 142]);
    let expected: Vec<_> = (0..144)
        .filter(|page| ![19, 2, 142].contains(page))
        .collect();
    assert_eq!(&first.cache_metadata()[4..], expected);
}

#[test]
fn finite_metadata_position_zero_requires_one_active_page() {
    let mut input = input(0);
    input.page_table.fill(u32::MAX);
    assert!(EngineeringTp2Finite2304MetadataV1::prepare(&input).is_err());
}

#[test]
fn finite_metadata_rejects_duplicate_active_pages() {
    let mut input = input(16);
    input.page_table[..2].copy_from_slice(&[7, 7]);
    assert!(EngineeringTp2Finite2304MetadataV1::prepare(&input).is_err());
}

#[test]
fn finite_metadata_rejects_out_of_range_and_max_active_pages() {
    for position in [0, 16, 2303] {
        for invalid in [144, u32::MAX] {
            let mut input = input(position);
            input.page_table[usize::try_from(position / 16).unwrap()] = invalid;
            assert!(EngineeringTp2Finite2304MetadataV1::prepare(&input).is_err());
        }
    }
}

#[test]
fn finite_metadata_rejects_wrong_page_extents_and_nonmax_tail() {
    for length in [0, 143, 145] {
        let mut input = input(0);
        input.page_table.resize(length, u32::MAX);
        assert!(EngineeringTp2Finite2304MetadataV1::prepare(&input).is_err());
    }
    for invalid in [0, 143, 144] {
        let mut input = input(0);
        input.page_table[143] = invalid;
        assert!(EngineeringTp2Finite2304MetadataV1::prepare(&input).is_err());
    }
}

#[test]
fn finite_metadata_rejects_wrong_geometry_and_position() {
    let mut input = input(0);
    input.geometry = EngineeringTp2GraphGeometryV1::Short64;
    assert!(EngineeringTp2Finite2304MetadataV1::prepare(&input).is_err());
    input.geometry = EngineeringTp2GraphGeometryV1::Long2304;
    for invalid in [2304, u32::MAX] {
        input.position = invalid;
        assert!(EngineeringTp2Finite2304MetadataV1::prepare(&input).is_err());
    }
}

#[test]
fn finite_metadata_new_active_page_does_not_remap_committed_prefix() {
    let mut earlier = input(15);
    earlier.page_table[0] = 57;
    let first = prepare(&earlier);
    let mut later = input(16);
    later.page_table[..2].copy_from_slice(&[57, 112]);
    let second = prepare(&later);
    assert_eq!(first.cache_metadata()[1], 57);
    assert_eq!(&second.cache_metadata()[1..3], &[57, 112]);
    // Prior inactive completion is not a reservation for the next logical page.
    assert_eq!(first.cache_metadata()[2], 0);
    assert_eq!(earlier.page_table[1], u32::MAX);
}

#[test]
fn finite_metadata_preserves_actual_rope_bytes_at_boundary_positions() {
    for position in [0, 1, 15, 16, 2047, 2303] {
        let input = input(position);
        let metadata = prepare(&input);
        let repacked: Vec<_> = metadata
            .rotary()
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect();
        assert_eq!(repacked, input.cos_sin);
        let (cos, sin) = rope_bytes(position, 1_000_000);
        assert_eq!(&repacked[..256], cos);
        assert_eq!(&repacked[256..], sin);
        if position == 0 {
            assert!(
                metadata.rotary()[..64]
                    .iter()
                    .all(|value| value.to_bits() == 1.0_f32.to_bits())
            );
            assert!(
                metadata.rotary()[64..]
                    .iter()
                    .all(|value| value.to_bits() == 0)
            );
        }
    }
}

#[test]
fn finite_metadata_rotary_packing_preserves_split_order_and_finite_bits() {
    let mut input = input(0);
    let bits: [u32; 128] = core::array::from_fn(|index| {
        if index < 64 {
            0x3e80_0000 + u32::try_from(index).unwrap()
        } else {
            0xbf00_0000 + u32::try_from(index).unwrap()
        }
    });
    input.cos_sin = bits.iter().flat_map(|word| word.to_le_bytes()).collect();
    let metadata = prepare(&input);
    assert_eq!(metadata.rotary().map(f32::to_bits), bits);
    for word in [0x0000_0001_u32, 0x8000_0000, 0x7f7f_ffff, 0xff7f_ffff] {
        input.cos_sin[64 * 4..65 * 4].copy_from_slice(&word.to_le_bytes());
        assert_eq!(prepare(&input).rotary()[64].to_bits(), word);
    }
}

#[test]
fn finite_metadata_rejects_wrong_rotary_byte_extents() {
    for length in [0, 256, 511, 513] {
        let mut input = input(0);
        input.cos_sin.resize(length, 0);
        assert!(EngineeringTp2Finite2304MetadataV1::prepare(&input).is_err());
    }
}

#[test]
fn finite_metadata_rejects_nonfinite_rotary_values_in_both_halves() {
    for index in [0, 63, 64, 127] {
        for word in [0x7f80_0000_u32, 0xff80_0000, 0x7fc0_1234, 0xff80_0001] {
            let mut input = input(0);
            input.cos_sin[index * 4..(index + 1) * 4].copy_from_slice(&word.to_le_bytes());
            assert!(EngineeringTp2Finite2304MetadataV1::prepare(&input).is_err());
        }
    }
}
