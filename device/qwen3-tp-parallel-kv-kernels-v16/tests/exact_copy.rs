//! CPU models of the checked scalar and lane schedules, not GPU execution.

use std::collections::BTreeSet;

const SENTINEL: u16 = 0x5aa5;

#[derive(Clone)]
struct Input {
    rows: u32,
    world: u32,
    max_pages: u32,
    physical_pages: u32,
    extent: usize,
    grid: [u32; 3],
    key: Vec<u16>,
    value: Vec<u16>,
    positions: Vec<u32>,
    table: Vec<u32>,
}

#[derive(Debug, Eq, PartialEq)]
enum Reject {
    Shape,
    Mapping,
    Duplicate,
}

fn columns(world: u32) -> Option<usize> {
    match world {
        1 => Some(1024),
        2 => Some(512),
        8 => Some(128),
        _ => None,
    }
}

fn fixture(rows: u32, world: u32, pages: u32) -> Input {
    let width = columns(world).unwrap();
    let mut input = Input {
        rows,
        world,
        max_pages: 4,
        physical_pages: pages,
        extent: 4096,
        grid: [64, 1, 1],
        key: (0..16 * width)
            .map(|i| (i as u16).wrapping_mul(257))
            .collect(),
        value: (0..16 * width)
            .map(|i| !(i as u16).wrapping_mul(509))
            .collect(),
        positions: vec![u32::MAX; 16],
        table: vec![u32::MAX; 16 * 4],
    };
    for row in 0..rows as usize {
        input.positions[row] = (row + 15) as u32;
        let logical = input.positions[row] as usize / 16;
        input.table[row * 4 + logical] = pages - 1;
    }
    input
}

fn validate(
    input: &Input,
    key_len: usize,
    value_len: usize,
) -> Result<(usize, Vec<usize>), Reject> {
    let width = columns(input.world).ok_or(Reject::Shape)?;
    let rows = input.rows as usize;
    let stride = input.max_pages as usize;
    let pages = input.physical_pages as usize;
    if rows == 0
        || rows > 16
        || stride == 0
        || stride > 512
        || pages == 0
        || pages > 512
        || input.key.len() < rows * width
        || input.key.len() > 16 * width
        || input.value.len() < rows * width
        || input.value.len() > 16 * width
        || input.positions.len() < rows
        || input.positions.len() > 16
        || input.table.len() < rows * stride
        || input.table.len() > 16 * stride
        || key_len != pages * 16 * width
        || value_len != pages * 16 * width
        || input.extent != 64
    {
        return Err(Reject::Shape);
    }
    let mut slots = Vec::new();
    for row in 0..rows {
        let position = input.positions[row] as usize;
        if position >= 8192 || position / 16 >= stride {
            return Err(Reject::Mapping);
        }
        let page = input.table[row * stride + position / 16] as usize;
        if page >= pages {
            return Err(Reject::Mapping);
        }
        let slot = page * 16 + position % 16;
        if slots.contains(&slot) {
            return Err(Reject::Duplicate);
        }
        slots.push(slot);
    }
    Ok((width, slots))
}

fn scalar(input: &Input, key: &mut [u16], value: &mut [u16]) -> Result<(), Reject> {
    let mut legacy = input.clone();
    legacy.extent = 64;
    let (width, slots) = validate(&legacy, key.len(), value.len())?;
    for (row, slot) in slots.into_iter().enumerate() {
        for component in 0..width {
            key[slot * width + component] = input.key[row * width + component];
            value[slot * width + component] = input.value[row * width + component];
        }
    }
    Ok(())
}

fn parallel(
    input: &Input,
    key: &mut [u16],
    value: &mut [u16],
    reverse: bool,
) -> Result<usize, Reject> {
    if input.rows != 1
        || input.world != 1
        || input.max_pages != 4
        || input.physical_pages != 4
        || input.extent != 4096
        || input.grid != [64, 1, 1]
    {
        return Err(Reject::Shape);
    }
    let mut legacy = input.clone();
    legacy.extent = 64;
    let (width, slots) = validate(&legacy, key.len(), value.len())?;
    let mut written = BTreeSet::new();
    let mut active_groups = BTreeSet::new();
    for invocation in 0..4096 {
        let invocation = if reverse {
            4095 - invocation
        } else {
            invocation
        };
        let output_slot = invocation / 64;
        let lane = invocation % 64;
        for (row, &slot) in slots.iter().enumerate() {
            if slot != output_slot {
                continue;
            }
            active_groups.insert(output_slot);
            for chunk in 0..16 {
                if chunk < width / 64 {
                    let component = lane + chunk * 64;
                    let source = row * width + component;
                    let destination = output_slot * width + lane + chunk * 64;
                    assert_eq!(destination % 64, lane);
                    assert!(written.insert(destination), "duplicate destination");
                    key[destination] = input.key[source];
                    value[destination] = input.value[source];
                }
            }
        }
    }
    assert_eq!(
        active_groups.len(),
        1,
        "exactly one selected physical slot group"
    );
    Ok(written.len())
}

fn compare(input: &Input) {
    let elements = input.physical_pages as usize * 16 * columns(input.world).unwrap();
    let mut expected_key = vec![SENTINEL; elements];
    let mut expected_value = expected_key.clone();
    scalar(input, &mut expected_key, &mut expected_value).unwrap();
    for reverse in [false, true] {
        let mut guarded_key = vec![SENTINEL; elements + 16];
        let mut guarded_value = guarded_key.clone();
        let writes = parallel(
            input,
            &mut guarded_key[8..8 + elements],
            &mut guarded_value[8..8 + elements],
            reverse,
        )
        .unwrap();
        assert_eq!(writes, input.rows as usize * columns(input.world).unwrap());
        assert_eq!(&guarded_key[8..8 + elements], expected_key);
        assert_eq!(&guarded_value[8..8 + elements], expected_value);
        for storage in [&guarded_key, &guarded_value] {
            assert_eq!(&storage[..8], &[SENTINEL; 8]);
            assert_eq!(&storage[8 + elements..], &[SENTINEL; 8]);
        }
    }
}

fn reject_unchanged(input: &Input, key_len: usize, value_len: usize) {
    let mut key = vec![SENTINEL; key_len];
    let mut value = vec![SENTINEL; value_len];
    assert!(parallel(input, &mut key, &mut value, false).is_err());
    assert!(key.iter().all(|&word| word == SENTINEL));
    assert!(value.iter().all(|&word| word == SENTINEL));
}

#[test]
fn expanded_row_world_profiles_reject_without_stores() {
    for world in [1, 2, 8] {
        for rows in 1..=16 {
            let input = fixture(rows, world, 4);
            if rows == 1 && world == 1 {
                compare(&input);
            } else {
                let extent = 4 * 16 * columns(world).unwrap();
                reject_unchanged(&input, extent, extent);
            }
        }
    }
}

#[test]
fn every_u16_bit_pattern_is_copied_without_numeric_conversion() {
    for start in (0..65536).step_by(1024) {
        let mut input = fixture(1, 1, 4);
        for column in 0..1024 {
            input.key[column] = (start + column) as u16;
            input.value[column] = !(start + column) as u16;
        }
        compare(&input);
    }
}

#[test]
fn every_logical_position_and_physical_page_preserves_complete_cache_bytes() {
    for position in 0..64 {
        for page in 0..4 {
            let mut input = fixture(1, 1, 4);
            input.positions[0] = position;
            input.table[position as usize / 16] = page;
            compare(&input);
        }
    }
}

#[test]
fn permuted_and_repeated_physical_page_tables_preserve_selected_slot() {
    for table in [[3, 0, 2, 1], [2, 2, 2, 2]] {
        for position in [0, 15, 16, 31, 32, 47, 48, 63] {
            let mut input = fixture(1, 1, 4);
            input.positions[0] = position;
            input.table[..4].copy_from_slice(&table);
            compare(&input);
        }
    }
}

#[test]
fn invalid_selected_metadata_rejects_before_any_workgroup_store() {
    for mutation in 0..4 {
        let mut input = fixture(1, 1, 4);
        match mutation {
            0 => input.positions[0] = 8192,
            1 => input.positions[0] = 64,
            2 => input.table[0] = u32::MAX,
            3 => input.table[0] = 4,
            _ => unreachable!(),
        }
        reject_unchanged(&input, 4 * 16 * 1024, 4 * 16 * 1024);
    }
}

#[test]
fn invalid_shapes_reject_before_copy() {
    for mutation in 0..22 {
        let mut input = fixture(1, 1, 4);
        match mutation {
            0 => input.rows = 0,
            1 => input.rows = 17,
            2 => input.rows = u32::MAX,
            3 => input.world = 0,
            4 => input.world = 3,
            5 => input.world = u32::MAX,
            6 => input.max_pages = 0,
            7 => input.max_pages = 513,
            8 => input.max_pages = u32::MAX,
            9 => input.physical_pages = 0,
            10 => input.physical_pages = 513,
            11 => input.physical_pages = u32::MAX,
            12 => input.extent = 0,
            13 => input.extent = 32,
            14 => input.extent = 128,
            15 => input.extent = 64,
            16 => input.max_pages = 3,
            17 => input.physical_pages = 3,
            18 => input.grid[0] = 63,
            19 => input.grid[0] = 65,
            20 => input.grid[1] = 2,
            21 => input.grid[2] = 2,
            _ => unreachable!(),
        }
        reject_unchanged(&input, 4 * 16 * 1024, 4 * 16 * 1024);
    }
}

#[test]
fn all_input_and_output_extent_limits_are_checked() {
    for mutation in 0..12 {
        let mut input = fixture(1, 1, 4);
        let mut key_len = 4 * 16 * 1024;
        let mut value_len = key_len;
        match mutation {
            0 => input.key.truncate(1023),
            1 => input.key.push(0),
            2 => input.value.truncate(1023),
            3 => input.value.push(0),
            4 => input.positions.clear(),
            5 => input.positions.push(0),
            6 => input.table.truncate(3),
            7 => input.table.push(0),
            8 => key_len -= 1,
            9 => key_len += 1,
            10 => value_len -= 1,
            11 => value_len += 1,
            _ => unreachable!(),
        }
        reject_unchanged(&input, key_len, value_len);
    }
}

#[test]
fn inactive_capacity_and_unselected_invalid_table_entries_are_not_read() {
    let input = fixture(1, 1, 4);
    compare(&input);
    let mut compact = input.clone();
    compact.key.truncate(1024);
    compact.value.truncate(1024);
    compact.positions.truncate(1);
    compact.table.truncate(4);
    compare(&compact);
}

#[test]
fn consecutive_positions_only_replace_their_selected_slot() {
    let mut input = fixture(1, 1, 4);
    let mut key = vec![SENTINEL; 4 * 16 * 1024];
    let mut value = key.clone();
    let mut expected_key = key.clone();
    let mut expected_value = key.clone();
    for position in [0, 1, 15, 16, 31, 32, 63] {
        input.positions[0] = position;
        input.table[..4].copy_from_slice(&[3, 0, 2, 1]);
        input.key[..1024].fill(position as u16);
        input.value[..1024].fill(!(position as u16));
        parallel(&input, &mut key, &mut value, false).unwrap();
        scalar(&input, &mut expected_key, &mut expected_value).unwrap();
        assert_eq!(key, expected_key);
        assert_eq!(value, expected_value);
    }
}
