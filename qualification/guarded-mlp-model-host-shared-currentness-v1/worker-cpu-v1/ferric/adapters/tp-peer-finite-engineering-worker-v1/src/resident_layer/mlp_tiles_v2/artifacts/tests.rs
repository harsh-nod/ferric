use super::*;

struct DecodeFake(Fake);
impl Loader for DecodeFake {
    type Kernel = usize;
    type State = usize;
    fn counts(&mut self) -> Result<Vec<usize>> {
        self.0.step("counts".into())?;
        Ok(vec![570, 566])
    }
    fn kernel(&mut self, r: usize, b: Vec<u8>, s: [u8; 32]) -> Result<(usize, KernelMetadataV1)> {
        self.0.kernel(r, b, s)
    }
    fn state(&mut self, _: usize) -> Result<usize> {
        panic!("decode image loading must not allocate comparison states")
    }
}
#[test]
fn decode_loads_only_two_kernels_before_replacement_state_roster() {
    let mut f = DecodeFake(fake());
    let (kernels, _) = load_decode(&mut f, image()).unwrap();
    assert_eq!(kernels, [0, 1]);
    assert_eq!(f.0.events, ["counts", "kernel0", "kernel1", "counts"]);
    for n in 0..4 {
        let mut f = DecodeFake(fake());
        f.0.fail = Some(n);
        assert!(load_decode(&mut f, image()).is_err());
        assert_eq!(f.0.events.len(), n + 1);
    }
    for rank in 0..2 {
        let mut f = DecodeFake(fake());
        f.0.bad = Some(rank);
        assert!(load_decode(&mut f, image()).is_err());
    }
    assert!(load_decode(&mut fake(), image()).is_err());
}
use fe2o3_kfd::engineering_wire::ExplicitArgumentV1;
fn metadata(sha: [u8; 32]) -> KernelMetadataV1 {
    KernelMetadataV1 {
        symbol: SYMBOL.into(),
        object_sha256: sha,
        kernarg_bytes: 344,
        kernarg_alignment: 8,
        group_segment_bytes: 512,
        private_segment_bytes: 0,
        wavefront_size: 64,
        implicit_argument_offset: Some(88),
        implicit_argument_bytes: 256,
        explicit_arguments: (0..11)
            .map(|i| ExplicitArgumentV1 {
                offset: i * 8,
                bytes: 8,
                global_buffer: true,
                pointee_alignment: Some(if i < 9 { 2 } else { 4 }),
                access: Some(if i < 5 {
                    Access::Read
                } else {
                    Access::ReadWrite
                }),
            })
            .collect(),
    }
}
fn image() -> Image {
    let bytes = b"synthetic custody, not executable admission".to_vec();
    let pin = crate::finite_forward_wire_v1::part(&bytes);
    Image::new(bytes, &pin).unwrap()
}
struct Fake {
    events: Vec<String>,
    fail: Option<usize>,
    count: usize,
    bad: Option<usize>,
}
impl Fake {
    fn step(&mut self, s: String) -> Result<()> {
        let n = self.events.len();
        self.events.push(s);
        if self.fail == Some(n) {
            Err("loader failure".into())
        } else {
            Ok(())
        }
    }
}
impl Loader for Fake {
    type Kernel = usize;
    type State = usize;
    fn counts(&mut self) -> Result<Vec<usize>> {
        self.step("counts".into())?;
        Ok(vec![
            714 + usize::from(self.count > 0),
            710 + usize::from(self.count > 1),
        ])
    }
    fn kernel(
        &mut self,
        rank: usize,
        _: Vec<u8>,
        sha: [u8; 32],
    ) -> Result<(usize, KernelMetadataV1)> {
        self.step(format!("kernel{rank}"))?;
        let mut m = metadata(sha);
        if self.bad == Some(rank) {
            m.group_segment_bytes = 0;
        }
        Ok((rank, m))
    }
    fn state(&mut self, rank: usize) -> Result<usize> {
        self.step(format!("state{rank}"))?;
        self.count += 1;
        Ok(rank)
    }
}
fn fake() -> Fake {
    Fake {
        events: vec![],
        fail: None,
        count: 0,
        bad: None,
    }
}
#[test]
fn exact_load_then_two_fresh_states_and_whole_group_count() {
    let mut f = fake();
    let (kernels, states, _) = load_all(&mut f, image()).unwrap();
    assert_eq!(kernels, [0, 1]);
    assert_eq!(states, [0, 1]);
    assert_eq!(
        f.events,
        ["counts", "kernel0", "kernel1", "state0", "state1", "counts"]
    );
}
#[test]
fn every_partial_load_and_allocation_failure_stops_without_fallback() {
    for n in 0..6 {
        let mut f = fake();
        f.fail = Some(n);
        assert!(load_all(&mut f, image()).is_err());
        assert_eq!(f.events.len(), n + 1);
    }
    for rank in 0..2 {
        let mut f = fake();
        f.bad = Some(rank);
        assert!(load_all(&mut f, image()).is_err());
        assert_eq!(f.count, 0);
    }
    let mut f = fake();
    f.count = 2;
    assert!(load_all(&mut f, image()).is_err());
    assert_eq!(f.events, ["counts"]);
}
#[test]
fn same_eighty_eight_byte_v1_abi_or_any_metadata_role_change_is_not_v2() {
    let good = metadata([7; 32]);
    validate_metadata(&good, [7; 32]).unwrap();
    for mutation in 0..10 {
        let mut m = good.clone();
        match mutation {
            0 => m.symbol = "ferric_qwen3_claimed_mlp_bf16_f32_v1".into(),
            1 => m.group_segment_bytes = 0,
            2 => m.private_segment_bytes = 4,
            3 => m.wavefront_size = 32,
            4 => m.kernarg_bytes = 88,
            5 => m.implicit_argument_offset = Some(80),
            6 => m.object_sha256 = [8; 32],
            7 => m.explicit_arguments[10].offset = 72,
            8 => m.explicit_arguments[10].access = Some(Access::Write),
            _ => m.explicit_arguments[9].pointee_alignment = Some(2),
        }
        assert!(validate_metadata(&m, [7; 32]).is_err());
    }
}
#[test]
fn actual_image_pin_is_required_but_never_claims_source_or_review_admission() {
    let bytes = b"unqualified synthetic bytes".to_vec();
    let mut pin = crate::finite_forward_wire_v1::part(&bytes);
    assert!(Image::new(bytes.clone(), &pin).is_ok());
    pin.sha256[0] ^= 1;
    assert!(Image::new(bytes.clone(), &pin).is_err());
    pin = crate::finite_forward_wire_v1::part(&bytes);
    pin.bytes += 1;
    assert!(Image::new(bytes, &pin).is_err());
    assert!(Image::new(vec![], &crate::finite_forward_wire_v1::part(&[])).is_err());
}
