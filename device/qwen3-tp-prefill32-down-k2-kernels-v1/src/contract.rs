pub const ROWS: usize = 32;
pub const N: usize = 4096;
pub const K: usize = 12288;
pub const GROUPS: usize = (ROWS / 16) * (N / 16);
pub const LANES: usize = 64;
pub const OUTPUT_BYTES: usize = ROWS * N * 4;

/// Geometry checks only, not pointer admission, source authentication or launch authority.
#[derive(Clone, Copy, Debug)]
pub struct LaunchContract {
    pub rows: u32,
    pub n: u32,
    pub k: u32,
    pub world_size: u32,
    pub projection: u32,
    pub a_elements: usize,
    pub weights_elements: usize,
    pub output_elements: usize,
    pub grid: [usize; 3],
    pub workgroup: [usize; 3],
}

impl LaunchContract {
    pub const fn exact() -> Self {
        Self {
            rows: 32,
            n: 4096,
            k: 12288,
            world_size: 1,
            projection: 2,
            a_elements: ROWS * K,
            weights_elements: K * N,
            output_elements: ROWS * N,
            grid: [GROUPS, 1, 1],
            workgroup: [LANES, 1, 1],
        }
    }

    pub const fn admitted(&self) -> bool {
        self.rows == 32
            && self.n == 4096
            && self.k == 12288
            && self.world_size == 1
            && self.projection == 2
            && self.a_elements == ROWS * K
            && self.weights_elements == K * N
            && self.output_elements == ROWS * N
            && self.grid[0] == GROUPS
            && self.grid[1] == 1
            && self.grid[2] == 1
            && self.workgroup[0] == LANES
            && self.workgroup[1] == 1
            && self.workgroup[2] == 1
    }
}

/// Host model of V5's unchanged M16/N16 tiled output map.
pub const fn output_coordinate(raw: usize, component: usize) -> Option<(usize, usize)> {
    if raw >= GROUPS * LANES || component >= 4 {
        return None;
    }
    let group = raw / LANES;
    let lane = raw % LANES;
    let row = (group / (N / 16)) * 16 + (lane / 16) * 4 + component;
    let column = (group % (N / 16)) * 16 + lane % 16;
    Some((row, column))
}
