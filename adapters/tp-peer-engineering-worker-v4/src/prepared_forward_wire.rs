//! Separate opt-in prepared interpreter framing; never a GPU scheduling API.

use super::{dependency_wire as dep, wire as legacy};
use fe2o3_kfd::engineering_wire::{
    self as base, CommandV1, KernelMetadataV1, ResponseV1, SequenceDispatchV1,
};
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

pub const PROTOCOL: u32 = 1;
pub const MODE: &str = "device-peer-tp2-prepared-interpreter-v1";
pub const PROFILE: &str = "qwen3-8b-tp2-mfma-baseline-attention-bf16-row1-cap16-context64-pages4";
pub const REFERENCE_SOURCE: &str =
    "fd5941a2eae2adc49373d0ae3df430084bbceea68ac7b5404931b7425fa36b78";
pub const NATIVE_SOURCE: &str = "62a8d989b5c7539f72e7b047283deb0baf9b95ef95b73ef1e5929e3a88021faa";
pub const MAIN_IMAGE: &str = "2e677384a5e86c6e1ae95f10333f2d0d330922ed4d1348acdb1b09f5655281e2";
pub const PEER_IMAGE: &str = "8436c1861dcc14f0346aeecc136e4187c459ac1c0b31778265c88b3d8c63396d";
pub const MAX_PROGRAM_BYTES: usize = 4 * 1024 * 1024;
pub const STEPS: usize = 1013;
pub const KERNEL_COUNTS: [u32; 2] = [616, 613];
pub const PACKET_COUNTS: [u32; 2] = [688, 685];
pub const VOCABULARY: u32 = 151_936;
pub const V22_IMAGE: &str = "ca58642a56ea4a522972952304d13e8dbfdf61fa85eb175d3b30c82db99839fc";
pub const V22_ROOT: &str = "ferric_qwen3_tp_single_wave_argmax_bf16_v22";
pub const V15_IMAGE: &str = "c33882db1afcd8eb26ec02bc43ea2144af322d29bf4e0e921e03ea74adc55af6";
pub const V15_ROOT: &str = "ferric_qwen3_tp_batch32_wave_rmsnorm_bf16_v15";
pub const SPLIT_ATTENTION_IMAGE: &str =
    "0110d14adc424ea3ff439c6dee8662a0af69bb738cdf05ca98aa5d2f4936f766";
pub const SPLIT_ATTENTION_ROOTS: [&str; 2] = [
    "ferric_qwen3_tp_split_context_partial_bf16_v1",
    "ferric_qwen3_tp_split_context_merge_bf16_v1",
];

/// Graph geometry is immutable; the interpreter's Program/Execute schemas stay short.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphGeometry {
    #[default]
    #[serde(rename = "short64")]
    Short64,
    #[serde(rename = "long2304")]
    Long2304,
}

impl GraphGeometry {
    pub const ALL: [Self; 2] = [Self::Short64, Self::Long2304];

    pub const fn argument(self) -> &'static str {
        match self {
            Self::Short64 => "short64",
            Self::Long2304 => "long2304",
        }
    }

    pub fn parse(value: &str) -> io::Result<Self> {
        match value {
            "short64" => Ok(Self::Short64),
            "long2304" => Ok(Self::Long2304),
            _ => Err(io::Error::other("unknown immutable graph geometry")),
        }
    }

    pub const fn context_tokens(self) -> u32 {
        match self {
            Self::Short64 => 64,
            Self::Long2304 => 2304,
        }
    }

    pub const fn pages(self) -> u32 {
        self.context_tokens() / 16
    }

    pub const fn attention_splits(self) -> u32 {
        match self {
            Self::Short64 => 1,
            Self::Long2304 => 18,
        }
    }

    pub const fn attention_scratch_values(self) -> u64 {
        16 * self.attention_splits() as u64 * 192
    }

    pub const fn metadata_lengths(self) -> [usize; 9] {
        let table = self.pages() as usize * 4;
        [4, table, 256, 256, 4, table, 256, 256, 4]
    }

    pub fn program_profile(self, profile: KernelProfile) -> String {
        match self {
            Self::Short64 => profile.program_profile().into(),
            Self::Long2304 => {
                let base =
                    "qwen3-8b-tp2-mfma-baseline-attention-bf16-row1-cap16-context2304-pages144";
                if profile == KernelProfile::Baseline {
                    base.into()
                } else {
                    format!("{base}+{}", profile.argument())
                }
            }
        }
    }
}

/// Graph-only, immutable kernel choice; baseline interpreter/native schemas stay closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KernelProfile {
    #[serde(rename = "baseline")]
    Baseline,
    #[serde(rename = "bf16-argmax-v22-scalar")]
    V22Scalar,
    #[serde(rename = "bf16-argmax-v22-wave")]
    V22Wave,
    #[serde(rename = "wave-stack-control")]
    WaveStackControl,
    #[serde(rename = "wave-stack-norm")]
    WaveStackNorm,
    #[serde(rename = "wave-stack-norm-attention")]
    WaveStackNormAttention,
    #[serde(rename = "wave-stack-norm-attention-kv")]
    WaveStackNormAttentionKv,
    #[serde(rename = "wave-stack-norm-attention-kv-mlp")]
    WaveStackNormAttentionKvMlp,
    #[serde(rename = "wave-stack-norm-split-attention-kv-mlp")]
    WaveStackNormSplitAttentionKvMlp,
}

impl KernelProfile {
    #[cfg(test)]
    pub const LEGACY_ALL: [Self; 8] = [
        Self::Baseline,
        Self::V22Scalar,
        Self::V22Wave,
        Self::WaveStackControl,
        Self::WaveStackNorm,
        Self::WaveStackNormAttention,
        Self::WaveStackNormAttentionKv,
        Self::WaveStackNormAttentionKvMlp,
    ];
    pub const ALL: [Self; 9] = [
        Self::Baseline,
        Self::V22Scalar,
        Self::V22Wave,
        Self::WaveStackControl,
        Self::WaveStackNorm,
        Self::WaveStackNormAttention,
        Self::WaveStackNormAttentionKv,
        Self::WaveStackNormAttentionKvMlp,
        Self::WaveStackNormSplitAttentionKvMlp,
    ];

    pub const fn argument(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::V22Scalar => "bf16-argmax-v22-scalar",
            Self::V22Wave => "bf16-argmax-v22-wave",
            Self::WaveStackControl => "wave-stack-control",
            Self::WaveStackNorm => "wave-stack-norm",
            Self::WaveStackNormAttention => "wave-stack-norm-attention",
            Self::WaveStackNormAttentionKv => "wave-stack-norm-attention-kv",
            Self::WaveStackNormAttentionKvMlp => "wave-stack-norm-attention-kv-mlp",
            Self::WaveStackNormSplitAttentionKvMlp => "wave-stack-norm-split-attention-kv-mlp",
        }
    }

    pub fn parse(value: &str) -> io::Result<Self> {
        match value {
            "baseline" => Ok(Self::Baseline),
            "bf16-argmax-v22-scalar" => Ok(Self::V22Scalar),
            "bf16-argmax-v22-wave" => Ok(Self::V22Wave),
            "wave-stack-control" => Ok(Self::WaveStackControl),
            "wave-stack-norm" => Ok(Self::WaveStackNorm),
            "wave-stack-norm-attention" => Ok(Self::WaveStackNormAttention),
            "wave-stack-norm-attention-kv" => Ok(Self::WaveStackNormAttentionKv),
            "wave-stack-norm-attention-kv-mlp" => Ok(Self::WaveStackNormAttentionKvMlp),
            "wave-stack-norm-split-attention-kv-mlp" => Ok(Self::WaveStackNormSplitAttentionKvMlp),
            _ => Err(io::Error::other("unknown graph kernel profile")),
        }
    }

    pub const fn has_v22(self) -> bool {
        matches!(self, Self::V22Scalar | Self::V22Wave) || self.has_v15()
    }

    pub const fn has_v15(self) -> bool {
        matches!(
            self,
            Self::WaveStackControl
                | Self::WaveStackNorm
                | Self::WaveStackNormAttention
                | Self::WaveStackNormAttentionKv
                | Self::WaveStackNormAttentionKvMlp
                | Self::WaveStackNormSplitAttentionKvMlp
        )
    }

    pub const fn wave_argmax(self) -> bool {
        matches!(self, Self::V22Wave) || self.has_v15()
    }

    pub const fn wave_hidden_norm(self) -> bool {
        self.has_v15() && !matches!(self, Self::WaveStackControl)
    }

    pub const fn wave_attention(self) -> bool {
        matches!(
            self,
            Self::WaveStackNormAttention
                | Self::WaveStackNormAttentionKv
                | Self::WaveStackNormAttentionKvMlp
        )
    }

    pub const fn wave_kv(self) -> bool {
        matches!(self, Self::WaveStackNormAttentionKv) || self.wave_mlp()
    }

    pub const fn wave_mlp(self) -> bool {
        matches!(
            self,
            Self::WaveStackNormAttentionKvMlp | Self::WaveStackNormSplitAttentionKvMlp
        )
    }

    pub const fn split_attention(self) -> bool {
        matches!(self, Self::WaveStackNormSplitAttentionKvMlp)
    }

    // Each of 36 layers adds one merge dispatch per rank. The collective
    // barriers stay unchanged; only the immutable split profile grows the graph.
    pub const fn steps(self) -> usize {
        if self.split_attention() { 1085 } else { STEPS }
    }

    pub const fn rank_dispatches(self) -> u32 {
        if self.split_attention() { 1013 } else { 941 }
    }

    pub const fn kernel_counts(self) -> [u32; 2] {
        if self.split_attention() {
            [652, 649]
        } else {
            KERNEL_COUNTS
        }
    }

    pub const fn graph_packet_counts(self) -> [u32; 2] {
        if self.split_attention() {
            [795, 793]
        } else {
            [759, 757]
        }
    }

    pub const fn collective_producer(self, collective: usize) -> usize {
        if self.split_attention() {
            11 + 22 * (collective / 2) + 8 * (collective % 2)
        } else {
            10 + 21 * (collective / 2) + 8 * (collective % 2)
        }
    }

    pub const fn program_profile(self) -> &'static str {
        match self {
            Self::Baseline => PROFILE,
            Self::V22Scalar => {
                "qwen3-8b-tp2-mfma-baseline-attention-bf16-row1-cap16-context64-pages4+bf16-argmax-v22-scalar"
            }
            Self::V22Wave => {
                "qwen3-8b-tp2-mfma-baseline-attention-bf16-row1-cap16-context64-pages4+bf16-argmax-v22-wave"
            }
            Self::WaveStackControl => {
                "qwen3-8b-tp2-mfma-baseline-attention-bf16-row1-cap16-context64-pages4+wave-stack-control"
            }
            Self::WaveStackNorm => {
                "qwen3-8b-tp2-mfma-baseline-attention-bf16-row1-cap16-context64-pages4+wave-stack-norm"
            }
            Self::WaveStackNormAttention => {
                "qwen3-8b-tp2-mfma-baseline-attention-bf16-row1-cap16-context64-pages4+wave-stack-norm-attention"
            }
            Self::WaveStackNormAttentionKv => {
                "qwen3-8b-tp2-mfma-baseline-attention-bf16-row1-cap16-context64-pages4+wave-stack-norm-attention-kv"
            }
            Self::WaveStackNormAttentionKvMlp => {
                "qwen3-8b-tp2-mfma-baseline-attention-bf16-row1-cap16-context64-pages4+wave-stack-norm-attention-kv-mlp"
            }
            Self::WaveStackNormSplitAttentionKvMlp => {
                "qwen3-8b-tp2-mfma-baseline-attention-bf16-row1-cap16-context64-pages4+wave-stack-norm-split-attention-kv-mlp"
            }
        }
    }

    pub const fn descriptors(self) -> usize {
        if self.split_attention() {
            41
        } else if self.has_v15() {
            37
        } else if self.has_v22() {
            35
        } else {
            34
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KernelBinding {
    pub id: u64,
    pub rank: u32,
    pub metadata: KernelMetadataV1,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BufferBinding {
    pub id: u64,
    pub rank: u32,
    pub bytes: u64,
    pub peer_readable: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub kernels: Vec<KernelBinding>,
    pub buffers: Vec<BufferBinding>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RankMetadata {
    pub positions: u64,
    pub page_table: u64,
    pub cos: u64,
    pub sin: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Template {
    pub dispatch: SequenceDispatchV1,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "step", rename_all = "snake_case", deny_unknown_fields)]
pub enum Step {
    Rank {
        rank: u32,
        template: Template,
    },
    Collective {
        layer: u32,
        operation: dep::Operation,
        producers: [Template; 2],
        consumers: Box<[Template; 2]>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Program {
    pub profile: String,
    pub reference_source_sha256: String,
    pub native_source_sha256: String,
    pub unique_ids: [u64; 2],
    pub group_id: u64,
    pub catalog: Catalog,
    pub token_buffer: u64,
    pub result_buffer: u64,
    pub metadata: [RankMetadata; 2],
    pub steps: Vec<Step>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Execute {
    pub id: u64,
    pub plan_sha256: [u8; 32],
    pub generation: u64,
    pub epoch: u64,
    pub token: u32,
    pub position: u32,
    pub page_table: [u32; 4],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "prepared_op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Setup {
        id: u64,
        rank: u32,
        peer_readable: bool,
        command: CommandV1,
    },
    Register {
        id: u64,
        program_sha256: [u8; 32],
        program_bytes: u32,
    },
    Execute {
        execution: Execute,
    },
    Close {
        id: u64,
    },
}

impl Request {
    pub fn id(&self) -> u64 {
        match self {
            Self::Setup { id, .. } | Self::Register { id, .. } | Self::Close { id } => *id,
            Self::Execute { execution } => execution.id,
        }
    }

    pub fn payload_bytes(&self) -> io::Result<usize> {
        if self.id() == 0 {
            return Err(io::Error::other("prepared request ID is zero"));
        }
        match self {
            Self::Setup {
                rank,
                peer_readable,
                command,
                ..
            } => {
                if *rank >= 2
                    || *peer_readable && !matches!(command, CommandV1::Allocate { .. })
                    || !matches!(
                        command,
                        CommandV1::Allocate { .. }
                            | CommandV1::Write { .. }
                            | CommandV1::LoadKernel { .. }
                            | CommandV1::ConfigurePerformance {
                                operational_currentness: false,
                                profile: false,
                                ..
                            }
                    )
                {
                    return Err(io::Error::other("prepared setup command scope"));
                }
                command.payload_bytes()
            }
            Self::Register { program_bytes, .. }
                if (1..=MAX_PROGRAM_BYTES).contains(&(*program_bytes as usize)) =>
            {
                Ok(*program_bytes as usize)
            }
            Self::Execute { .. } => Ok(512),
            Self::Close { .. } => Ok(0),
            Self::Register { .. } => Err(io::Error::other("prepared program framing bound")),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReceipt {
    pub id: u64,
    pub plan_sha256: [u8; 32],
    pub generation: u64,
    pub epoch: u64,
    pub position: u32,
    pub input_token: u32,
    pub output_token: u32,
    pub unique_ids: [u64; 2],
    pub kernel_counts: [u32; 2],
    pub barrier_counts: [u32; 2],
    pub packet_counts: [u32; 2],
    pub ordinary_completions: u32,
    pub collectives: Vec<dep::Receipt>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "prepared_op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Response {
    Ready {
        protocol: u32,
        mode: String,
        profile: String,
        unique_ids: [u64; 2],
        process_id: u32,
        authority: String,
        currentness: String,
        control_allocation_flags: u32,
    },
    Setup {
        id: u64,
        response: ResponseV1,
    },
    Registered {
        id: u64,
        plan_sha256: [u8; 32],
        catalog_sha256: [u8; 32],
        steps: u32,
        kernel_counts: [u32; 2],
    },
    Executed {
        receipt: ExecutionReceipt,
    },
    Closed {
        id: u64,
    },
    Fatal {
        id: u64,
        message: String,
    },
}

pub fn read_request(input: &mut impl Read) -> io::Result<Option<(Request, Vec<u8>)>> {
    let Some(header) = base::read_header_v1::<Request>(input)? else {
        return Ok(None);
    };
    let mut payload = vec![0; header.payload_bytes()?];
    input.read_exact(&mut payload)?;
    Ok(Some((header, payload)))
}

pub fn write_request(output: &mut impl Write, header: &Request, payload: &[u8]) -> io::Result<()> {
    if header.payload_bytes()? != payload.len() {
        return Err(io::Error::other("prepared request payload mismatch"));
    }
    base::write_header_v1(output, header)?;
    output.write_all(payload)?;
    output.flush()
}

pub fn write_response(output: &mut impl Write, header: &Response) -> io::Result<()> {
    base::write_header_v1(output, header)?;
    output.flush()
}

pub fn read_response(input: &mut impl Read) -> io::Result<Option<Response>> {
    base::read_header_v1(input)
}

pub fn setup_request(
    rank: u32,
    peer_readable: bool,
    command: CommandV1,
    internal_id: u64,
) -> legacy::Request {
    legacy::Request {
        id: internal_id,
        rank,
        peer_readable,
        command,
        round_ranks: Vec::new(),
    }
}
