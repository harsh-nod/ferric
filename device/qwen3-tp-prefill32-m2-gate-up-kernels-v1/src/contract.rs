pub const ROWS: usize = 32;
pub const N: usize = 12288;
pub const K: usize = 4096;
pub const GROUPS: usize = N / 16;
pub const LANES: usize = 64;
pub const HALF_ELEMENTS: usize = 16 * N;
pub const HALF_BYTES: usize = HALF_ELEMENTS * 2;
pub const OUTPUT_BYTES: usize = ROWS * N * 2;

#[derive(Clone, Copy, Debug)]
pub struct LaunchContract {
    pub rows: u32,
    pub n: u32,
    pub k: u32,
    pub world_size: u32,
    pub projection: u32,
    pub a_elements: usize,
    pub weights_elements: usize,
    pub output_elements: [usize; 2],
    pub output_buffers: [u64; 2],
    pub output_offsets: [usize; 2],
    pub output_allocation_bytes: usize,
    pub grid_x: usize,
    pub block_x: usize,
}

impl LaunchContract {
    pub const fn exact(projection: u32, output_buffer: u64) -> Self {
        Self {
            rows: 32,
            n: 12288,
            k: 4096,
            world_size: 1,
            projection,
            a_elements: ROWS * K,
            weights_elements: K * N,
            output_elements: [HALF_ELEMENTS; 2],
            output_buffers: [output_buffer; 2],
            output_offsets: [0, HALF_BYTES],
            output_allocation_bytes: OUTPUT_BYTES,
            grid_x: GROUPS,
            block_x: LANES,
        }
    }

    pub const fn admitted(&self) -> bool {
        self.rows == 32
            && self.n == 12288
            && self.k == 4096
            && self.world_size == 1
            && (self.projection == 4 || self.projection == 5)
            && self.a_elements == ROWS * K
            && self.weights_elements == K * N
            && self.output_elements[0] == HALF_ELEMENTS
            && self.output_elements[1] == HALF_ELEMENTS
            && self.output_buffers[0] == self.output_buffers[1]
            && self.output_offsets[0] == 0
            && self.output_offsets[1] == HALF_BYTES
            && self.output_allocation_bytes == OUTPUT_BYTES
            && self.grid_x == GROUPS
            && self.block_x == LANES
    }
}

/// Host model of the existing M16 typed view, shifted by one output half.
/// This is a contract check, not execution of a device ownership witness.
pub const fn output_coordinate(
    raw: usize,
    half: usize,
    component: usize,
) -> Option<(usize, usize)> {
    if raw >= GROUPS * LANES || half >= 2 || component >= 4 {
        return None;
    }
    let group = raw / LANES;
    let lane = raw % LANES;
    let row = half * 16 + (lane / 16) * 4 + component;
    let column = group * 16 + lane % 16;
    Some((row, column))
}
