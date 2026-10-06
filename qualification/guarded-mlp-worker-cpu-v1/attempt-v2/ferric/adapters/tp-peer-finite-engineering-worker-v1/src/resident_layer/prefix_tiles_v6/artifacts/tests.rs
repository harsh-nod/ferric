use super::*;
use fe2o3_kfd::engineering_wire::ExplicitArgumentV1;
fn metadata() -> KernelMetadataV1 {
    KernelMetadataV1 {
        symbol: SYMBOL.into(),
        object_sha256: [3; 32],
        kernarg_bytes: 376,
        kernarg_alignment: 8,
        wavefront_size: 64,
        private_segment_bytes: 0,
        group_segment_bytes: 512,
        implicit_argument_offset: Some(120),
        implicit_argument_bytes: 256,
        explicit_arguments: (0..15)
            .map(|i| ExplicitArgumentV1 {
                offset: i * 8,
                bytes: 8,
                global_buffer: true,
                pointee_alignment: Some(if matches!(i, 4 | 5 | 13 | 14) { 4 } else { 2 }),
                access: Some(if i < 7 {
                    Access::Read
                } else {
                    Access::ReadWrite
                }),
            })
            .collect(),
    }
}
#[test]
fn prefix_layer_image_custody_requires_exact_nonempty_bytes() {
    let b = b"synthetic test image, no executable authority".to_vec();
    let pin = Part {
        bytes: b.len() as u32,
        sha256: Sha256::digest(&b).into(),
    };
    assert_eq!(Image::new(b.clone(), &pin).unwrap().sha256(), pin.sha256);
    assert!(Image::new(Vec::new(), &pin).is_err());
    let mut wrong = b.clone();
    wrong[0] ^= 1;
    assert!(Image::new(wrong, &pin).is_err());
    assert!(
        Image::new(
            b,
            &Part {
                bytes: pin.bytes + 1,
                sha256: pin.sha256
            }
        )
        .is_err()
    );
}
#[test]
fn prefix_layer_rejects_same_width_v5_and_wrong_resources() {
    validate(&metadata(), [3; 32]).unwrap();
    for field in 0..9 {
        let mut m = metadata();
        match field {
            0 => m.symbol = "ferric_qwen3_claimed_rmsnorm_qkv_attention_output_bf16_f32_v5".into(),
            1 => m.object_sha256 = [4; 32],
            2 => m.kernarg_bytes = 344,
            3 => m.group_segment_bytes = 0,
            4 => m.private_segment_bytes = 4,
            5 => m.wavefront_size = 32,
            6 => m.implicit_argument_offset = Some(88),
            7 => m.implicit_argument_bytes = 0,
            _ => m.kernarg_alignment = 4,
        }
        assert!(validate(&m, [3; 32]).is_err(), "field {field}");
    }
}
#[test]
fn prefix_layer_metadata_checks_every_pointer_role_and_alignment() {
    for index in 0..15 {
        for field in 0..5 {
            let mut m = metadata();
            let a = &mut m.explicit_arguments[index];
            match field {
                0 => a.offset += 8,
                1 => a.bytes = 4,
                2 => a.global_buffer = false,
                3 => a.pointee_alignment = Some(8),
                _ => a.access = Some(Access::Write),
            }
            assert!(validate(&m, [3; 32]).is_err(), "role {index} field {field}");
        }
    }
    let mut m = metadata();
    m.explicit_arguments.pop();
    assert!(validate(&m, [3; 32]).is_err());
}
