//! Structural wire projection only. Source allocation numbers are not native IDs.
use super::*;
use crate::finite_composition_wire as wire;
use crate::tp_execution::{EngineeringTpArgumentV1 as Arg, EngineeringTpBufferAccessV1 as Access};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn buffer(value: EngineeringTp2FiniteBufferV1) -> wire::Buffer {
    wire::Buffer {
        rank: value.rank(),
        id: value.id(),
        elements: value.elements() as u64,
        element_bytes: value.element_bytes(),
    }
}

fn weight(kind: Qwen3TensorKind) -> TpResult<wire::WeightKind> {
    use Qwen3TensorKind as S;
    use wire::WeightKind as W;
    Ok(match kind {
        S::InputLayerNorm => W::InputLayerNorm,
        S::QueryProjection => W::QueryProjection,
        S::KeyProjection => W::KeyProjection,
        S::ValueProjection => W::ValueProjection,
        S::QueryNorm => W::QueryNorm,
        S::KeyNorm => W::KeyNorm,
        S::OutputProjection => W::OutputProjection,
        S::PostAttentionLayerNorm => W::PostAttentionLayerNorm,
        S::GateProjection => W::GateProjection,
        S::UpProjection => W::UpProjection,
        S::DownProjection => W::DownProjection,
        _ => return Err("finite wire unsupported original layer role".into()),
    })
}

fn command(value: &EngineeringTpDispatchV1) -> Value {
    let arguments: Vec<_> = value
        .arguments
        .iter()
        .map(|arg| match *arg {
            Arg::Buffer {
                id,
                offset,
                elements,
                element_bytes,
                access,
            } => json!({
                "kind": "buffer", "source_id": id, "offset": offset,
                "elements": elements, "element_bytes": element_bytes,
                "access": match access { Access::Read => "read", Access::Write => "write",
                    Access::ReadWrite => "read_write" },
            }),
            Arg::U32(value) => json!({"kind": "u32", "value": value}),
            Arg::F32(value) => json!({"kind": "f32_bits", "value": value.to_bits()}),
        })
        .collect();
    json!({"symbol": value.kernel, "grid_workgroups": value.grid_workgroups,
        "workgroup_size": value.workgroup_size, "arguments": arguments})
}

impl EngineeringTp2FiniteCompositionV1<'_> {
    /// Versioned structured JSON snapshot of the actual source grammar. Exact F32
    /// bits and ordered argument/step arrays are retained. This includes queued
    /// transposed source IDs; it is not a finite-native executable or authority.
    /// # Errors
    /// Rejects serialization failure or a snapshot beyond the fixed wire bound.
    pub fn source_program_snapshot_v1(&self) -> TpResult<Vec<u8>> {
        let steps: Vec<_> = self.source_program.steps.iter().map(|step| match step {
            Step::Rank { rank, dispatch } => json!({"kind": "rank", "rank": rank,
                "dispatch": command(dispatch)}),
            Step::Collective(value) => json!({"kind": "collective", "rows": value.rows,
                "group_id": value.key.group_id, "epoch": value.key.epoch,
                "layer": value.key.layer, "model": match value.key.model_role {
                    ferric_spec::Qwen3ModelRole::Target8B => "target8b",
                    ferric_spec::Qwen3ModelRole::Draft06B => "draft06b",
                },
                "operation": match value.key.operation {
                    Qwen3TensorParallelCollectiveV1::AttentionOutputSum => "attention_output_sum",
                    Qwen3TensorParallelCollectiveV1::FeedForwardDownSum => "feed_forward_down_sum",
                },
                "producers": value.producers.iter().map(command).collect::<Vec<_>>(),
                "consumers": value.consumers.iter().map(command).collect::<Vec<_>>(),
            }),
        }).collect();
        let metadata: Vec<_> = self.source_program.metadata.iter().map(|m| json!({
            "positions": m.positions, "page_table": m.page_table, "cos": m.cos, "sin": m.sin,
        })).collect();
        let snapshot = json!({"schema": "ferric-finite-source-grammar-v1",
            "profile": Self::PROFILE, "group_id": self.source_program.group_id,
            "token_buffer": self.source_program.token_buffer,
            "result_buffer": self.source_program.result_buffer, "metadata": metadata,
            "steps": steps});
        let bytes = serde_json::to_vec(&snapshot).map_err(|e| e.to_string())?;
        if bytes.is_empty() || bytes.len() > wire::MAX_REGISTRATION {
            return Err("finite source snapshot bound".into());
        }
        Ok(bytes)
    }

    /// Project exact source-owned records to the shared inert DTO and validate
    /// its closed rosters. No native allocation, capability or state is imported.
    /// # Errors
    /// Rejects an unsupported role, invalid roster or source snapshot failure.
    pub fn wire_registration_v1(&self) -> TpResult<wire::Registration> {
        let layers = self
            .layers
            .iter()
            .map(|layer| {
                Ok(wire::Layer {
                    rank: layer.rank,
                    layer: layer.layer,
                    weights: layer
                        .weights
                        .iter()
                        .map(|&(kind, b)| {
                            Ok(wire::Weight {
                                kind: weight(kind)?,
                                buffer: buffer(b),
                            })
                        })
                        .collect::<TpResult<Vec<_>>>()?,
                    caches: layer.caches.map(buffer),
                })
            })
            .collect::<TpResult<Vec<_>>>()?;
        let globals = self
            .globals
            .iter()
            .map(|&(kind, b)| {
                Ok(wire::Global {
                    kind: match kind {
                        Qwen3TensorKind::TokenEmbedding => wire::GlobalKind::TokenEmbedding,
                        Qwen3TensorKind::FinalNorm => wire::GlobalKind::FinalNorm,
                        Qwen3TensorKind::LanguageModelHead => wire::GlobalKind::LanguageModelHead,
                        _ => return Err("finite wire unsupported global role".into()),
                    },
                    buffer: buffer(b),
                })
            })
            .collect::<TpResult<Vec<_>>>()?;
        let auxiliary = self
            .auxiliary
            .iter()
            .map(|&(kind, b)| {
                use EngineeringTp2FiniteAuxiliaryKindV1 as S;
                use wire::AuxiliaryKind as W;
                wire::Auxiliary {
                    kind: match kind {
                        S::QueryProjection => W::QueryProjection,
                        S::KeyProjection => W::KeyProjection,
                        S::ValueProjection => W::ValueProjection,
                        S::QueryNormalized => W::QueryNormalized,
                        S::KeyNormalized => W::KeyNormalized,
                        S::KeyRotated => W::KeyRotated,
                        S::Cosine => W::Cosine,
                        S::Sine => W::Sine,
                        S::Empty => W::Empty,
                        S::Token => W::Token,
                        S::Logits => W::Logits,
                        S::Choice => W::Choice,
                        S::Positions => W::Positions,
                        S::PageTable => W::PageTable,
                    },
                    buffer: buffer(b),
                }
            })
            .collect();
        let scratch = self
            .scratch
            .iter()
            .map(|&(kind, b)| {
                use EngineeringTp2FiniteScratchRoleV1 as S;
                use wire::ScratchKind as W;
                wire::Scratch {
                    kind: match kind {
                        S::Hidden => W::Hidden,
                        S::PostAttentionResidual => W::PostAttentionResidual,
                        S::Normalized => W::Normalized,
                        S::Query => W::Query,
                        S::Attention => W::Attention,
                        S::Gate => W::Gate,
                        S::Up => W::Up,
                        S::Activation => W::Activation,
                        S::Partial => W::Partial,
                    },
                    buffer: buffer(b),
                }
            })
            .collect();
        let pending_buffers = self
            .pending_buffers
            .iter()
            .map(|b| {
                use EngineeringTp2FinitePendingBufferKindV1 as S;
                use wire::PendingKind as W;
                wire::PendingBuffer {
                    rank: b.rank,
                    layer: b.layer,
                    kind: match b.kind {
                        S::PackedQkvWeight => W::PackedQkvWeight,
                        S::PackedHeadNormWeight => W::PackedHeadNormWeight,
                        S::QkvOutput => W::QkvOutput,
                        S::Rotary => W::Rotary,
                        S::CacheMetadata => W::CacheMetadata,
                    },
                    elements: b.elements as u64,
                    element_bytes: b.element_bytes,
                }
            })
            .collect();
        let state_slots = self
            .state_slots
            .iter()
            .map(|s| wire::StateSlot {
                forward: s.forward,
                layer: s.layer,
                rank: s.rank,
                kind: match s.kind {
                    EngineeringTp2FiniteStateKindV1::PrefixV5 => wire::StateKind::PrefixV5,
                    EngineeringTp2FiniteStateKindV1::MlpV1 => wire::StateKind::MlpV1,
                },
                atomic_words: s.atomic_words() as u32,
            })
            .collect();
        let source = self.source_program_snapshot_v1()?;
        let value = wire::Registration {
            profile: Self::PROFILE.into(),
            bundle_id: self.bundle_id,
            model_id: self.model_id,
            session: self.session,
            pool_identity: self.pool_identity,
            group_id: self.group_id,
            child_identity: self.child_identity,
            layers,
            globals,
            auxiliary,
            scratch,
            pending_buffers,
            state_slots,
            source_program_bytes: source.len() as u32,
            source_program_sha256: Sha256::digest(&source).into(),
        };
        value.validate().map_err(|e| e.to_string())?;
        Ok(value)
    }
}
