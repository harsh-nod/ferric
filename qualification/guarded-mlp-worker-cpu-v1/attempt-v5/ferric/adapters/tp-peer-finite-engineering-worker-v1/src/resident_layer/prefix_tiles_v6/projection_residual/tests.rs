use super::*;
use fe2o3_kfd::engineering_wire::ExplicitArgumentV1;
fn image() -> Image {
    let bytes = b"synthetic candidate, not executable".to_vec();
    let pin = Part {
        bytes: bytes.len() as u32,
        sha256: Sha256::digest(&bytes).into(),
    };
    Image::new(bytes, &pin).unwrap()
}
fn metadata(sha: [u8; 32]) -> KernelMetadataV1 {
    KernelMetadataV1 {
        symbol: SYMBOL.into(),
        object_sha256: sha,
        kernarg_bytes: 424,
        kernarg_alignment: 8,
        wavefront_size: 64,
        private_segment_bytes: 0,
        group_segment_bytes: 0,
        implicit_argument_offset: Some(168),
        implicit_argument_bytes: 256,
        explicit_arguments: (0..22)
            .map(|i| {
                let pointer = i < 20 && i % 2 == 0;
                ExplicitArgumentV1 {
                    offset: if i < 20 { i * 8 } else { 160 + (i - 20) * 4 },
                    bytes: if i < 20 { 8 } else { 4 },
                    global_buffer: pointer,
                    pointee_alignment: pointer.then_some(if i < 16 { 4 } else { 2 }),
                    access: pointer.then_some(if i == 18 { Access::Write } else { Access::Read }),
                }
            })
            .collect(),
    }
}
#[test]
fn projection_image_requires_exact_actual_bytes() {
    let i = image();
    let pin = Part {
        bytes: i.bytes.len() as u32,
        sha256: i.sha256,
    };
    assert!(Image::new(Vec::new(), &pin).is_err());
    let mut wrong = i.bytes.clone();
    wrong[0] ^= 1;
    assert!(Image::new(wrong, &pin).is_err());
    assert!(
        Image::new(
            i.bytes,
            &Part {
                bytes: pin.bytes + 1,
                ..pin
            }
        )
        .is_err()
    );
}
#[test]
fn projection_metadata_rejects_old_symbol_and_every_abi_role() {
    let sha = image().sha256();
    validate(&metadata(sha), sha).unwrap();
    for field in 0..9 {
        let mut m = metadata(sha);
        match field {
            0 => m.symbol = "ferric_qwen3_tp_peer_tp2_ordered_residual_bf16_v18".into(),
            1 => m.object_sha256 = [9; 32],
            2 => m.kernarg_bytes = 168,
            3 => m.kernarg_alignment = 4,
            4 => m.wavefront_size = 32,
            5 => m.private_segment_bytes = 4,
            6 => m.group_segment_bytes = 4,
            7 => m.implicit_argument_offset = Some(0),
            _ => m.implicit_argument_bytes = 0,
        }
        assert!(validate(&m, sha).is_err());
    }
    for index in 0..22 {
        for field in 0..5 {
            let mut m = metadata(sha);
            let a = &mut m.explicit_arguments[index];
            match field {
                0 => a.offset += 4,
                1 => a.bytes += 4,
                2 => a.global_buffer = !a.global_buffer,
                3 => a.pointee_alignment = Some(16),
                _ => a.access = Some(Access::ReadWrite),
            }
            assert!(validate(&m, sha).is_err(), "{index}/{field}");
        }
    }
}
struct Fake {
    calls: usize,
    fail: Option<usize>,
    bad_owner: Option<usize>,
    bad_meta: Option<usize>,
    bad_count: Option<usize>,
}
impl Fake {
    fn new() -> Self {
        Self {
            calls: 0,
            fail: None,
            bad_owner: None,
            bad_meta: None,
            bad_count: None,
        }
    }
    fn step(&mut self) -> Result<usize> {
        let n = self.calls;
        self.calls += 1;
        if self.fail == Some(n) {
            Err("injected load".into())
        } else {
            Ok(n)
        }
    }
}
impl Loader for Fake {
    type Token = usize;
    fn counts(&mut self) -> Result<Vec<usize>> {
        let n = self.step()?;
        Ok(if self.bad_count == Some(n) {
            vec![713, 710]
        } else {
            vec![714, 710]
        })
    }
    fn load(&mut self, rank: usize, image: &Image) -> Result<(usize, usize, KernelMetadataV1)> {
        self.step()?;
        let mut m = metadata(image.sha256);
        if self.bad_meta == Some(rank) {
            m.symbol = "old".into();
        }
        Ok((
            rank,
            if self.bad_owner == Some(rank) {
                1 - rank
            } else {
                rank
            },
            m,
        ))
    }
}
#[test]
fn projection_load_preserves_census_and_returns_only_complete_pair() {
    let mut f = Fake::new();
    assert_eq!(load_pair(&mut f, &image()).unwrap(), [0, 1]);
    assert_eq!(f.calls, 4);
    for fail in 0..4 {
        let mut f = Fake::new();
        f.fail = Some(fail);
        assert!(load_pair(&mut f, &image()).is_err());
        assert_eq!(f.calls, fail + 1);
    }
    for rank in 0..2 {
        for owner in [false, true] {
            let mut f = Fake::new();
            if owner {
                f.bad_owner = Some(rank)
            } else {
                f.bad_meta = Some(rank)
            }
            assert!(load_pair(&mut f, &image()).is_err());
            assert_eq!(f.calls, rank + 2);
        }
    }
    for n in [0, 3] {
        let mut f = Fake::new();
        f.bad_count = Some(n);
        assert!(load_pair(&mut f, &image()).is_err());
        assert_eq!(f.calls, n + 1);
    }
}
#[test]
fn projection_residual_selection_never_falls_back_or_changes_original_pair() {
    let original = [10, 11];
    let selected = [20, 21];
    for _first in [true, false] {
        for rank in 0..2 {
            assert_eq!(
                *super::super::select_residual(Some(&selected), rank, |_| panic!("fallback"))
                    .unwrap(),
                selected[rank]
            );
            assert_eq!(
                *super::super::select_residual(None, rank, |r| Ok(&original[r])).unwrap(),
                original[rank]
            );
        }
    }
    assert!(super::super::select_residual(Some(&selected), 2, |_| panic!("fallback")).is_err());
    assert_eq!(original, [10, 11]);
}
