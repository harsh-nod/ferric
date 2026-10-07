use super::*;

#[test]
fn exact_model_rank_geometry_and_projection_axes() {
    for role in [1, 2] {
        let full = geometry(role, 1).unwrap();
        for world in [1, 2, 8] {
            let local = geometry(role, world).unwrap();
            assert_eq!(local.hidden, full.hidden);
            assert_eq!(local.query_heads * world, full.query_heads);
            assert_eq!(local.kv_heads * world, 8);
            assert_eq!(local.intermediate * world, full.intermediate);
            for (op, n) in [
                (1, local.query_heads * 128),
                (2, local.kv_heads * 128),
                (3, local.kv_heads * 128),
                (4, local.intermediate),
                (5, local.intermediate),
            ] {
                assert!(column_shape(n, full.hidden, role, world, op));
                assert!(!column_shape(n + 1, full.hidden, role, world, op));
                assert!(!column_shape(n, full.hidden + 1, role, world, op));
            }
            assert!(partial_shape(
                full.hidden,
                local.query_heads * 128,
                role,
                world,
                1
            ));
            assert!(partial_shape(
                full.hidden,
                local.intermediate,
                role,
                world,
                2
            ));
            assert!(!partial_shape(
                full.hidden + 1,
                local.intermediate,
                role,
                world,
                2
            ));
        }
    }
    for role in [0, 3, u32::MAX] {
        assert_eq!(geometry(role, 1), None);
    }
    for world in [0, 3, 4, 16, u32::MAX] {
        assert_eq!(geometry(1, world), None);
    }
    assert!(!column_shape(4_096, 4_096, 1, 1, 0));
    assert!(!partial_shape(4_096, 4_096, 1, 1, 3));
}

#[test]
fn capacity_and_initialized_prefix_boundaries_fail_closed() {
    assert!(append_position(0, 1));
    assert!(append_position(8_191, 8_192));
    assert!(!append_position(1, 1));
    assert!(!append_position(0, 0));
    assert!(!append_position(0, 8_193));
    assert!(attention_prefix(1, 1));
    assert!(attention_prefix(8_192, 8_192));
    assert!(!attention_prefix(0, 1));
    assert!(!attention_prefix(2, 1));
    assert!(!attention_prefix(1, 8_193));
    assert!(!attention_prefix(u32::MAX, u32::MAX));
}
