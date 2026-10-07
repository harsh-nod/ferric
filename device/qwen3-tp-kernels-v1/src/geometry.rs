macro_rules! tp_role_world_valid_v1 {
    ($role:expr, $world:expr) => {
        ($role == 1 || $role == 2) && ($world == 1 || $world == 2 || $world == 8)
    };
}

macro_rules! tp_sizes_v1 {
    ($role:expr, $world:expr) => {
        (
            if $role == 1 { 4_096 } else { 1_024 },
            (if $role == 1 { 32 } else { 16 }) / $world,
            8 / $world,
            (if $role == 1 { 12_288 } else { 3_072 }) / $world,
        )
    };
}

macro_rules! tp_column_shape_v1 {
    ($n:expr, $k:expr, $role:expr, $world:expr, $op:expr) => {{
        let (hidden, queries, keys, intermediate) = tp_sizes_v1!($role, $world);
        $k == hidden
            && (($op == 1 && $n == queries * 128)
                || (($op == 2 || $op == 3) && $n == keys * 128)
                || (($op == 4 || $op == 5) && $n == intermediate))
    }};
}

macro_rules! tp_partial_shape_v1 {
    ($n:expr, $k:expr, $role:expr, $world:expr, $op:expr) => {{
        let (hidden, queries, _, intermediate) = tp_sizes_v1!($role, $world);
        $n == hidden && (($op == 1 && $k == queries * 128) || ($op == 2 && $k == intermediate))
    }};
}

macro_rules! tp_capacity_valid_v1 {
    ($capacity:expr) => {
        $capacity > 0 && $capacity <= 8_192
    };
}
