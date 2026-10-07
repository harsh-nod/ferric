//! Independent bounded coordinate/bit-copy models; these do not execute device authority.

#[derive(Clone)]
struct Fixture {
    key: Vec<u16>,
    value: Vec<u16>,
    position: u32,
    page: u32,
    pages: u32,
    grid: [u32; 3],
}

fn fixture() -> Fixture {
    Fixture {
        key: vec![0; 1024],
        value: vec![0; 1024],
        position: 0,
        page: 0,
        pages: 1,
        grid: [16, 1, 1],
    }
}

fn copy(input: &Fixture, key: &mut [u16], value: &mut [u16]) -> Result<usize, ()> {
    if input.position >= 8192
        || input.pages == 0
        || input.pages > 512
        || input.page >= input.pages
        || input.key.len() != 1024
        || input.value.len() != 1024
        || key.len() != 1024
        || value.len() != 1024
        || input.grid != [16, 1, 1]
    {
        return Err(());
    }
    let mut seen = std::collections::BTreeSet::new();
    for group in 0..16 {
        for lane in 0..64 {
            let index = group * 64 + lane;
            assert!(index < 1024 && seen.insert(index));
            key[index] = input.key[index];
            value[index] = input.value[index];
        }
    }
    Ok(seen.len())
}

#[test]
fn copy_v19_preserves_every_u16_encoding_in_both_outputs() {
    for chunk in 0..64 {
        let mut input = fixture();
        for index in 0..1024 {
            input.key[index] = u16::try_from(chunk * 1024 + index).unwrap();
            input.value[index] = !input.key[index];
        }
        let before = (input.key.clone(), input.value.clone());
        let mut key = vec![0x55aa; 1024];
        let mut value = vec![0x55aa; 1024];
        assert_eq!(copy(&input, &mut key, &mut value), Ok(1024));
        assert_eq!((key, value), before);
        assert_eq!((input.key, input.value), before);
    }
}

#[test]
fn copy_v19_boundary_slots_leave_prefix_suffix_and_other_pages_untouched() {
    for (position, page, pages) in [
        (0, 0, 1),
        (15, 0, 1),
        (16, 1, 2),
        (31, 0, 3),
        (8191, 511, 512),
    ] {
        let mut input = fixture();
        input.position = position;
        input.page = page;
        input.pages = pages;
        input.key.fill(0x7fc1);
        input.value.fill(0x8000);
        let offset = (page as usize * 16 + (position % 16) as usize) * 1024;
        let length = pages as usize * 16 * 1024;
        let mut key = vec![0x55aa; length + 8];
        let mut value = key.clone();
        let begin = offset + 4;
        copy(
            &input,
            &mut key[begin..begin + 1024],
            &mut value[begin..begin + 1024],
        )
        .unwrap();
        for (actual, expected) in [(&key, 0x7fc1), (&value, 0x8000)] {
            assert!(
                actual[begin..begin + 1024]
                    .iter()
                    .all(|&word| word == expected)
            );
            assert!(
                actual[..begin]
                    .iter()
                    .chain(&actual[begin + 1024..])
                    .all(|&word| word == 0x55aa)
            );
        }
    }
}

#[test]
fn copy_v19_shape_and_scalar_rejections_leave_every_output_unchanged() {
    for mutation in 0..22 {
        let mut input = fixture();
        let mut key = vec![0x55aa; 1024];
        let mut value = key.clone();
        match mutation {
            0 => input.position = 8192,
            1 => input.position = u32::MAX,
            2 => input.pages = 0,
            3 => input.pages = 513,
            4 => input.page = 1,
            5 => input.page = u32::MAX,
            6 => {
                input.key.pop();
            }
            7 => input.key.push(0),
            8 => {
                input.value.pop();
            }
            9 => input.value.push(0),
            10 => {
                key.pop();
            }
            11 => key.push(0x55aa),
            12 => {
                value.pop();
            }
            13 => value.push(0x55aa),
            14 => input.grid = [1, 1, 1],
            15 => input.grid = [17, 1, 1],
            16 => input.grid = [16, 2, 1],
            17 => input.grid = [16, 1, 2],
            18 => input.key.clear(),
            19 => input.value.clear(),
            20 => key.clear(),
            21 => value.clear(),
            _ => unreachable!(),
        }
        let before = (key.clone(), value.clone());
        assert_eq!(copy(&input, &mut key, &mut value), Err(()));
        assert_eq!((key, value), before);
    }
}
