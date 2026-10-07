//! Independent coordinate/bit models, not execution of the device kernel or pool.

const ROWS: usize = 16;
const WIDTH: usize = 1024;
const WORDS: usize = ROWS * WIDTH;
const GUARD: u16 = 0x55aa;

#[derive(Clone)]
struct Input {
    key: Vec<u16>,
    value: Vec<u16>,
    first_position: u32,
    page: u32,
    pages: u32,
    rotation: u32,
    grid: [u32; 3],
}

fn fixture(rotation: u32, encoding_chunk: usize) -> Input {
    let mut input = Input {
        key: vec![0; WORDS],
        value: vec![0; WORDS],
        first_position: 112,
        page: 2,
        pages: 4,
        rotation,
        grid: [256, 1, 1],
    };
    let mut positions = (0..ROWS).collect::<Vec<_>>();
    if rotation == 1 {
        positions.rotate_right(1);
    }
    for (source_row, position) in positions.into_iter().enumerate() {
        for component in 0..WIDTH {
            let bits =
                u16::try_from(encoding_chunk * WORDS + position * WIDTH + component).unwrap();
            input.key[source_row * WIDTH + component] = bits;
            input.value[source_row * WIDTH + component] = !bits;
        }
    }
    input
}

fn model(input: &Input, key: &mut [u16], value: &mut [u16]) -> Result<usize, ()> {
    if input.first_position > 8176
        || input.first_position & 15 != 0
        || input.pages == 0
        || input.pages > 512
        || input.page >= input.pages
        || input.rotation > 1
        || input.grid != [256, 1, 1]
        || input.key.len() != WORDS
        || input.value.len() != WORDS
        || key.len() != WORDS
        || value.len() != WORDS
    {
        return Err(());
    }
    let mut destinations = std::collections::BTreeSet::new();
    let mut sources = std::collections::BTreeSet::new();
    for group in 0..256 {
        for lane in 0..64 {
            let index = group * 64 + lane;
            let row = index / WIDTH;
            let source_row = if input.rotation == 0 {
                row
            } else if row == 15 {
                0
            } else {
                row + 1
            };
            let source_index = source_row * WIDTH + index % WIDTH;
            assert!(destinations.insert(index) && sources.insert(source_index));
            key[index] = input.key[source_index];
            value[index] = input.value[source_index];
        }
    }
    assert_eq!(destinations, sources);
    Ok(destinations.len())
}

#[test]
fn every_u16_encoding_survives_both_orders_in_both_outputs() {
    for rotation in 0..=1 {
        for chunk in 0..4 {
            let input = fixture(rotation, chunk);
            let before = (input.key.clone(), input.value.clone());
            let mut key = vec![GUARD; WORDS];
            let mut value = key.clone();
            assert_eq!(model(&input, &mut key, &mut value), Ok(WORDS));
            for (index, (&key, &value)) in key.iter().zip(&value).enumerate() {
                let expected = u16::try_from(chunk * WORDS + index).unwrap();
                assert_eq!((key, value), (expected, !expected));
            }
            assert_eq!((input.key, input.value), before);
        }
    }
}

#[test]
fn final_prompt_rotation_places_the_published_row_in_the_last_slot() {
    let input = fixture(1, 0);
    let mut key = vec![GUARD; WORDS];
    let mut value = key.clone();
    model(&input, &mut key, &mut value).unwrap();
    assert_eq!(key[15 * WIDTH..], input.key[..WIDTH]);
    assert_eq!(key[..15 * WIDTH], input.key[WIDTH..]);
    assert_eq!(value[15 * WIDTH..], input.value[..WIDTH]);
    assert_eq!(value[..15 * WIDTH], input.value[WIDTH..]);
}

#[test]
fn boundary_pages_preserve_immutable_prefix_other_pages_and_guards() {
    for (first, page, pages) in [(0, 0, 1), (16, 1, 3), (112, 2, 4), (8176, 511, 512)] {
        let mut input = fixture(1, 0);
        input.first_position = first;
        input.page = page;
        input.pages = pages;
        let begin = 4 + usize::try_from(page).unwrap() * WORDS;
        let length = 8 + usize::try_from(pages).unwrap() * WORDS;
        let mut key = vec![GUARD; length];
        let mut value = key.clone();
        let before = (input.key.clone(), input.value.clone());
        model(
            &input,
            &mut key[begin..begin + WORDS],
            &mut value[begin..begin + WORDS],
        )
        .unwrap();
        for actual in [&key, &value] {
            assert!(
                actual[..begin]
                    .iter()
                    .chain(&actual[begin + WORDS..])
                    .all(|&word| word == GUARD)
            );
        }
        assert_eq!((input.key, input.value), before);
    }
}

#[test]
fn invalid_scalars_extents_and_launches_do_not_write() {
    for mutation in 0..25 {
        let mut input = fixture(0, 0);
        let mut key = vec![GUARD; WORDS];
        let mut value = key.clone();
        match mutation {
            0 => input.first_position = 1,
            1 => input.first_position = 8177,
            2 => input.first_position = 8192,
            3 => input.first_position = u32::MAX,
            4 => input.pages = 0,
            5 => input.pages = 513,
            6 => input.page = input.pages,
            7 => input.page = u32::MAX,
            8 => input.rotation = 2,
            9 => input.rotation = u32::MAX,
            10 => {
                input.key.pop();
            }
            11 => input.key.push(0),
            12 => {
                input.value.pop();
            }
            13 => input.value.push(0),
            14 => {
                key.pop();
            }
            15 => key.push(GUARD),
            16 => {
                value.pop();
            }
            17 => value.push(GUARD),
            18 => input.key.clear(),
            19 => input.value.clear(),
            20 => key.clear(),
            21 => value.clear(),
            22 => input.grid = [255, 1, 1],
            23 => input.grid = [257, 1, 1],
            24 => input.grid = [128, 2, 1],
            _ => unreachable!(),
        }
        let before = (key.clone(), value.clone());
        assert_eq!(model(&input, &mut key, &mut value), Err(()));
        assert_eq!((key, value), before);
    }
}

#[derive(Clone)]
struct Row {
    position: u32,
    page: u32,
    sequence: u64,
}

// Prospective selector model only. Real integration must retain the pool's
// sealed reservation, all slot/page/COW checks and exact typed view admission.
fn select(rows: &[Row], exclusive: bool, world: u32, prefill: bool) -> Option<(u32, u32, u32)> {
    if rows.len() != ROWS || !exclusive || world != 1 || !prefill {
        return None;
    }
    let first = rows.iter().map(|row| row.position).min()?;
    if first > 8176 || first & 15 != 0 {
        return None;
    }
    for rotation in 0..=1 {
        let mut expected = (first..first + 16).collect::<Vec<_>>();
        if rotation == 1 {
            expected.rotate_right(1);
        }
        if rows.iter().zip(expected).all(|(row, position)| {
            row.position == position && row.page == rows[0].page && row.sequence == rows[0].sequence
        }) {
            return Some((first, rows[0].page, rotation));
        }
    }
    None
}

#[test]
fn page_selection_matches_all_eight_matched_prompt_chunks_and_final_reordering() {
    for first in (0..128).step_by(16) {
        let mut rows = (first..first + 16)
            .map(|position| Row {
                position,
                page: 7 - first / 16,
                sequence: 91,
            })
            .collect::<Vec<_>>();
        let rotation = u32::from(first == 112);
        if rotation == 1 {
            rows.rotate_right(1);
        }
        assert_eq!(
            select(&rows, true, 1, true),
            Some((first, 7 - first / 16, rotation))
        );
    }
}

#[test]
fn selector_falls_back_for_unaligned_incomplete_mixed_or_nonexclusive_rows() {
    let base = (16..32)
        .map(|position| Row {
            position,
            page: 5,
            sequence: 91,
        })
        .collect::<Vec<_>>();
    for mutation in 0..10 {
        let mut rows = base.clone();
        let (mut exclusive, mut world, mut prefill) = (true, 1, true);
        match mutation {
            0 => {
                rows.pop();
            }
            1 => rows.push(rows[0].clone()),
            2 => rows[0].position = rows[1].position,
            3 => rows[0].page = 4,
            4 => rows[0].sequence = 92,
            5 => rows.swap(0, 1),
            6 => {
                for row in &mut rows {
                    row.position += 1;
                }
            }
            7 => exclusive = false,
            8 => world = 8,
            9 => prefill = false,
            _ => unreachable!(),
        }
        assert!(select(&rows, exclusive, world, prefill).is_none());
    }
}
