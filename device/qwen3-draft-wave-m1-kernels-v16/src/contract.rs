//! Inert launch metadata for explicit engineering integration, never launch authority.

pub const ROOTS_V16: [&str; 2] = [
    "ferric_qwen3_draft_wave_m1_gemv_bf16_v16",
    "ferric_qwen3_draft_wave_m1_gemv_f32_v16",
];
pub const TARGET: &str = "gfx950:xnack-";
pub const WORKGROUP: [u32; 3] = [64, 1, 1];
pub const MAX_GRID_WORKGROUPS: [u32; 2] = [3072, 151936];
pub const EXPLICIT_ARGUMENT_BYTES: u32 = 68;
pub const CAPACITY_ROWS: u32 = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionRole {
    Query,
    Key,
    Value,
    Gate,
    Up,
    AttentionOutput,
    Down,
    Head,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputCarrier {
    Bf16,
    F32,
}

impl OutputCarrier {
    pub const fn element_bytes(self) -> u32 {
        match self {
            Self::Bf16 => 2,
            Self::F32 => 4,
        }
    }
}

/// Host integration must also bind the compiler roster and checked device buffers.
/// All lengths here are element counts, not byte counts; weights remain [N,K].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectionDescriptor {
    kernel: &'static str,
    output: OutputCarrier,
    grid: [u32; 3],
    /// Kernel scalar order after the three slices: rows, n, k, world, projection.
    scalars: [u32; 5],
}

impl ProjectionRole {
    /// Deliberately rejects multi-row work and every target/TP-sharded profile.
    pub const fn descriptor(self, rows: u32, world: u32) -> Option<ProjectionDescriptor> {
        if rows != 1 || world != 1 {
            return None;
        }
        let (n, k, tag, output) = match self {
            Self::Query => (2048, 1024, 1, OutputCarrier::Bf16),
            Self::Key => (1024, 1024, 2, OutputCarrier::Bf16),
            Self::Value => (1024, 1024, 3, OutputCarrier::Bf16),
            Self::Gate => (3072, 1024, 4, OutputCarrier::Bf16),
            Self::Up => (3072, 1024, 5, OutputCarrier::Bf16),
            Self::AttentionOutput => (1024, 2048, 1, OutputCarrier::F32),
            Self::Down => (1024, 3072, 2, OutputCarrier::F32),
            Self::Head => (151936, 1024, 6, OutputCarrier::F32),
        };
        let kernel = match output {
            OutputCarrier::Bf16 => ROOTS_V16[0],
            OutputCarrier::F32 => ROOTS_V16[1],
        };
        Some(ProjectionDescriptor {
            kernel,
            output,
            grid: [n, 1, 1],
            scalars: [rows, n, k, world, tag],
        })
    }
}

impl ProjectionDescriptor {
    pub const fn kernel(self) -> &'static str {
        self.kernel
    }

    pub const fn output(self) -> OutputCarrier {
        self.output
    }

    pub const fn grid(self) -> [u32; 3] {
        self.grid
    }

    pub const fn scalars(self) -> [u32; 5] {
        self.scalars
    }

    /// Supports the existing v10 resident capacity without reading or writing its tail.
    pub const fn accepts_lengths(self, a: u64, weights: u64, output: u64) -> bool {
        let n = self.scalars[1] as u64;
        let k = self.scalars[2] as u64;
        a >= k
            && a <= CAPACITY_ROWS as u64 * k
            && weights == n * k
            && output >= n
            && output <= CAPACITY_ROWS as u64 * n
    }
}
