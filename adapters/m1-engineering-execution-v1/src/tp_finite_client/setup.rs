//! Parent projection of source-owned recipes into the closed setup wire.
use super::process::OwnedChild;
use super::{Images, Result, hash, require};
use crate::finite_composition_wire as source;
use crate::finite_setup_wire_v1 as wire;
use crate::tp_execution::batched::{
    EngineeringTp2FiniteUploadKeyV1 as UploadKey, EngineeringTp2FiniteUploadV1 as Upload,
};
use sha2::Digest;

pub(super) fn key(value: UploadKey) -> wire::Key {
    match value {
        UploadKey::Source { rank, id, .. } => wire::Key::Source { rank, id },
        UploadKey::PackedQkv { rank, layer } => wire::Key::Pending {
            rank,
            layer: Some(layer),
            role: source::PendingKind::PackedQkvWeight,
        },
        UploadKey::PackedHeadNorm { rank, layer } => wire::Key::Pending {
            rank,
            layer: Some(layer),
            role: source::PendingKind::PackedHeadNormWeight,
        },
    }
}

pub(super) fn scope(registration: &source::Registration) -> wire::Scope {
    wire::Scope {
        bundle_id: registration.bundle_id,
        model_id: registration.model_id,
        session: registration.session,
        pool_identity: registration.pool_identity,
        group_id: registration.group_id,
        child_identity: registration.child_identity,
    }
}

pub(super) fn manifest(
    uploads: &[Upload<'_>],
    tail: wire::TailHeadTranspose,
) -> Result<wire::UploadManifest> {
    require(uploads.len() == 939, "parent upload recipe count")?;
    let rows: Vec<_> = uploads
        .iter()
        .map(|u| wire::Upload {
            key: key(u.key()),
            bytes: u.bytes() as u64,
            sha256: u.sha256(),
        })
        .collect();
    for (index, row) in rows.iter().enumerate() {
        require(
            row.sha256 != [0; 32]
                && row.bytes != 0
                && !rows[..index].iter().any(|v| v.key == row.key),
            "parent duplicate/empty recipe",
        )?;
    }
    require(
        rows.iter().any(|v| {
            v.key == tail.source && v.sha256 == tail.source_sha256 && v.bytes == tail.bytes
        }),
        "parent tail original-source recipe join",
    )?;
    Ok(wire::UploadManifest {
        version: 1,
        uploads: rows,
        tail: Some(tail),
    })
}

pub(super) fn mutable_keys(registration: &source::Registration) -> Result<Vec<wire::Key>> {
    let source_key = |b: source::Buffer| wire::Key::Source {
        rank: b.rank,
        id: b.id,
    };
    let mut keys = Vec::with_capacity(196);
    for layer in &registration.layers {
        keys.extend(layer.caches.map(source_key));
    }
    keys.extend(registration.scratch.iter().map(|v| source_key(v.buffer)));
    keys.extend(registration.auxiliary.iter().map(|v| source_key(v.buffer)));
    for p in &registration.pending_buffers {
        if !matches!(
            p.kind,
            source::PendingKind::PackedQkvWeight | source::PendingKind::PackedHeadNormWeight
        ) {
            keys.push(wire::Key::Pending {
                rank: p.rank,
                layer: p.layer,
                role: p.kind,
            });
        }
    }
    require(keys.len() == 196, "parent mutable roster cardinality")?;
    for (index, key) in keys.iter().enumerate() {
        require(!keys[..index].contains(key), "parent mutable key alias")?;
    }
    Ok(keys)
}

pub(super) struct Stream<'a> {
    child: &'a mut OwnedChild,
    devices: [u64; 2],
    session: [u8; 32],
    next: u64,
    allocated_ids: Vec<u64>,
}

fn check_ack(
    id: u64,
    expected: wire::Status,
    response: &wire::Response,
    allocated: &[u64],
) -> Result<()> {
    require(
        response.protocol == wire::PROTOCOL
            && response.id == id
            && response.status == expected
            && response.native_opened
            && !response.gpu_execution
            && !response.forward_started
            && !response.production_authority,
        "parent setup acknowledgement phase/identity/claims",
    )?;
    match (expected, response.catalog_id) {
        (wire::Status::Allocated, Some(id)) => require(
            id != 0 && !allocated.contains(&id),
            "parent repeated native allocation identity",
        ),
        (wire::Status::Allocated, None) | (_, Some(_)) => {
            Err("parent unexpected allocation acknowledgement".into())
        }
        (_, None) => Ok(()),
    }
}
impl<'a> Stream<'a> {
    pub(super) fn begin(
        child: &'a mut OwnedChild,
        devices: [u64; 2],
        registration: &source::Registration,
        program: &[u8],
        manifest: &wire::UploadManifest,
        images: &Images,
    ) -> Result<Self> {
        registration.validate().map_err(|e| e.to_string())?;
        require(
            registration.child_identity == child.id()
                && registration.source_program_sha256 == hash(program)
                && registration.source_program_bytes as usize == program.len(),
            "parent exact source snapshot join",
        )?;
        let encoded_registration = serde_json::to_vec(registration).map_err(|e| e.to_string())?;
        let encoded_manifest = serde_json::to_vec(manifest).map_err(|e| e.to_string())?;
        let slices = [
            encoded_registration.as_slice(),
            program,
            encoded_manifest.as_slice(),
            images.prefix.as_slice(),
            images.mlp.as_slice(),
            images.residual.as_slice(),
            images.tail.as_slice(),
        ];
        let parts: Vec<_> = slices
            .iter()
            .map(|b| wire::Part {
                bytes: b.len() as u32,
                sha256: hash(b),
            })
            .collect();
        let command = wire::Command::Begin(wire::Begin {
            scope: scope(registration),
            registration: parts[0],
            source_program: parts[1],
            uploads: parts[2],
            prefix_image: parts[3],
            mlp_image: parts[4],
            residual_image: parts[5],
            tail_image: Some(parts[6]),
        });
        let request = wire::Request {
            protocol: wire::PROTOCOL,
            id: 1,
            device_ids: devices,
            session: registration.session,
            command,
        };
        let payload = slices.concat();
        require(
            request.payload_bytes().map_err(|e| e.to_string())? == payload.len(),
            "parent Begin bound",
        )?;
        let mut stream = Self {
            child,
            devices,
            session: registration.session,
            next: 1,
            allocated_ids: Vec::new(),
        };
        stream.exchange(request.command, &payload, wire::Status::OwnerCreated)?;
        Ok(stream)
    }

    fn exchange(
        &mut self,
        command: wire::Command,
        payload: &[u8],
        expected: wire::Status,
    ) -> Result<()> {
        self.child.check_deadline()?;
        let request = wire::Request {
            protocol: wire::PROTOCOL,
            id: self.next,
            device_ids: self.devices,
            session: self.session,
            command,
        };
        wire::write_request(
            self.child.input.as_mut().ok_or("closed setup stdin")?,
            &request,
            payload,
        )
        .map_err(|e| e.to_string())?;
        let response =
            wire::read_response(self.child.output.as_mut().ok_or("closed setup stdout")?)
                .map_err(|e| e.to_string())?
                .ok_or("EOF before setup acknowledgement")?;
        check_ack(self.next, expected, &response, &self.allocated_ids)?;
        if let Some(id) = response.catalog_id {
            self.allocated_ids.push(id);
        }
        self.next = self
            .next
            .checked_add(1)
            .ok_or("setup command counter overflow")?;
        self.child.check_deadline()
    }
    pub(super) fn upload(&mut self, upload: &Upload<'_>) -> Result<()> {
        let key = key(upload.key());
        self.exchange(
            wire::Command::Allocate { key },
            &[],
            wire::Status::Allocated,
        )?;
        let mut seen = 0_usize;
        let mut digest = sha2::Sha256::new();
        upload.visit_chunks(source::MAX_TRANSFER, |offset, bytes| {
            require(offset == seen, "parent recipe chunk order")?;
            self.exchange(
                wire::Command::Write {
                    key,
                    offset: offset as u64,
                    part: wire::Part {
                        bytes: bytes.len() as u32,
                        sha256: hash(bytes),
                    },
                },
                bytes,
                wire::Status::Uploaded,
            )?;
            digest.update(bytes);
            seen += bytes.len();
            Ok(())
        })?;
        let observed: [u8; 32] = digest.finalize().into();
        require(
            seen == upload.bytes() && observed == upload.sha256(),
            "parent recipe changed while streaming",
        )
    }
    pub(super) fn zero(&mut self, key: wire::Key) -> Result<()> {
        self.exchange(
            wire::Command::Allocate { key },
            &[],
            wire::Status::Allocated,
        )
    }
    pub(super) fn allocate_tail(&mut self) -> Result<()> {
        self.exchange(
            wire::Command::AllocateTailHead,
            &[],
            wire::Status::TailHeadAllocated,
        )
    }
    pub(super) fn write_tail(&mut self, offset: usize, bytes: &[u8]) -> Result<()> {
        self.exchange(
            wire::Command::WriteTailHead {
                offset: offset as u64,
                part: wire::Part {
                    bytes: bytes.len() as u32,
                    sha256: hash(bytes),
                },
            },
            bytes,
            wire::Status::TailHeadUploaded,
        )
    }
    pub(super) fn seal(mut self) -> Result<u64> {
        require(
            self.allocated_ids.len() == 1135,
            "parent setup allocation count",
        )?;
        self.exchange(
            wire::Command::LoadArtifacts,
            &[],
            wire::Status::ArtifactsLoaded,
        )?;
        self.exchange(
            wire::Command::BindAndSeal,
            &[],
            wire::Status::LayersAndTailSealed,
        )?;
        Ok(self.next - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_recipe_keys_never_become_imported_catalog_ids() {
        assert_eq!(
            key(UploadKey::Source {
                rank: 1,
                id: 717,
                layer: 2,
                kind: ferric_spec::Qwen3TensorKind::QueryProjection
            }),
            wire::Key::Source { rank: 1, id: 717 }
        );
        assert_eq!(
            key(UploadKey::PackedQkv { rank: 1, layer: 2 }),
            wire::Key::Pending {
                rank: 1,
                layer: Some(2),
                role: source::PendingKind::PackedQkvWeight
            }
        );
        assert_eq!(
            key(UploadKey::PackedHeadNorm { rank: 0, layer: 35 }),
            wire::Key::Pending {
                rank: 0,
                layer: Some(35),
                role: source::PendingKind::PackedHeadNormWeight
            }
        );
    }
    #[test]
    fn setup_acknowledgements_reject_replay_claims_and_reused_owner_ids() {
        let good = wire::Response {
            protocol: wire::PROTOCOL,
            id: 2,
            status: wire::Status::Allocated,
            catalog_id: Some(101),
            native_opened: true,
            gpu_execution: false,
            forward_started: false,
            production_authority: false,
        };
        check_ack(2, wire::Status::Allocated, &good, &[]).unwrap();
        assert!(check_ack(2, wire::Status::Allocated, &good, &[101]).is_err());
        for mutation in 0..7 {
            let mut response = good.clone();
            match mutation {
                0 => response.id += 1,
                1 => response.status = wire::Status::LayersAndTailSealed,
                2 => response.catalog_id = None,
                3 => response.native_opened = false,
                4 => response.gpu_execution = true,
                5 => response.forward_started = true,
                6 => response.production_authority = true,
                _ => unreachable!(),
            }
            assert!(check_ack(2, wire::Status::Allocated, &response, &[]).is_err());
        }
    }
}
