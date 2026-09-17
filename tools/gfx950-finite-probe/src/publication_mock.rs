use crate::publication_artifact as abi;
use crate::session::{Packet, Transport};
use crate::Result;
use fe2o3_kfd::engineering_wire::{CommandV1, ResponseV1};
use std::collections::BTreeMap;

pub struct Mock {
    pub commands: u32,
    pub allocations: u64,
    pub dispatches: u32,
    pub reads: u32,
    pub frees: Vec<u64>,
    pub closed: bool,
    buffers: BTreeMap<u64, Vec<u8>>,
    mutation: u32,
    case: abi::Case,
}

impl Mock {
    pub fn new(mutation: u32, case: abi::Case) -> Self {
        Self {
            commands: 0,
            allocations: 0,
            dispatches: 0,
            reads: 0,
            frees: vec![],
            closed: false,
            buffers: BTreeMap::new(),
            mutation,
            case,
        }
    }
    fn word(&mut self, buffer: u64, cell: usize, bits: u32) {
        let start = abi::GUARD + cell * 4;
        self.buffers.get_mut(&buffer).unwrap()[start..start + 4]
            .copy_from_slice(&bits.to_le_bytes());
    }
    fn complete(&mut self) {
        let inputs = self.buffers[&3][abi::GUARD..abi::GUARD + 512].to_vec();
        for cell in 0..256 {
            self.word(
                4,
                cell,
                if self.case.shape == abi::Shape::Valid {
                    u32::from(cell < 128)
                } else {
                    3
                },
            );
            self.word(5, cell, 0);
        }
        if self.case.shape == abi::Shape::Valid {
            for (cell, bytes) in inputs.chunks_exact(4).enumerate() {
                let bits = u32::from_le_bytes(bytes.try_into().unwrap());
                self.word(1, cell, bits);
                self.word(2, cell, if cell % 2 == 0 { 2 } else { 1 });
                if cell % 2 == 0 {
                    self.word(4, 128 + cell, 2);
                    self.word(5, 128 + cell, bits);
                }
            }
        }
        if self.mutation == 5 {
            self.buffers.get_mut(&3).unwrap()[abi::GUARD] ^= 1;
        }
        if (6..=15).contains(&self.mutation) {
            let index = usize::try_from(self.mutation - 6).unwrap();
            let offset = if index % 2 == 0 {
                0
            } else {
                abi::GUARD + abi::BYTES[index / 2]
            };
            self.buffers.get_mut(&(index as u64 / 2 + 1)).unwrap()[offset] ^= 1;
        }
        if self.mutation == 21 {
            self.word(5, 128, 0);
        }
    }
}

impl Transport for Mock {
    fn request(&mut self, command: CommandV1, payload: Vec<u8>) -> Result<Packet> {
        self.commands += 1;
        assert_eq!(command.payload_bytes().unwrap(), payload.len());
        let mut response_payload = vec![];
        let response = match command {
            CommandV1::LoadKernel { symbol, .. } => {
                assert_eq!(symbol, abi::SYMBOL);
                let mut metadata = super::metadata();
                if self.mutation == 1 {
                    metadata.object_sha256[0] ^= 1;
                }
                ResponseV1::LoadedKernel {
                    kernel: if self.mutation == 19 { 0 } else { 7 },
                    metadata,
                }
            }
            CommandV1::Allocate { bytes } => {
                self.allocations += 1;
                let buffer = if self.mutation == 2 {
                    1
                } else {
                    self.allocations
                };
                self.buffers
                    .insert(buffer, vec![0; usize::try_from(bytes).unwrap()]);
                ResponseV1::Allocated {
                    buffer,
                    bytes: bytes + u64::from(self.mutation == 3),
                }
            }
            CommandV1::Write { buffer, offset, .. } => {
                assert_eq!(self.dispatches, 0, "no host write after dispatch");
                assert_eq!(offset, 0);
                assert_eq!(payload.len(), self.buffers[&buffer].len());
                *self.buffers.get_mut(&buffer).unwrap() = payload;
                if self.mutation == 4 {
                    ResponseV1::Freed
                } else {
                    ResponseV1::Written
                }
            }
            CommandV1::Dispatch {
                kernel,
                workgroup,
                grid,
                pointers,
                timeout_ms,
                ..
            } => {
                assert_eq!(self.dispatches, 0);
                self.dispatches += 1;
                assert_eq!(kernel, 7);
                assert_eq!(workgroup, abi::WORKGROUP);
                assert_eq!(grid, abi::GRID);
                assert_eq!(timeout_ms, 10_000);
                assert_eq!(
                    payload,
                    abi::kernarg(&super::metadata(), &self.case).unwrap()
                );
                assert_eq!(pointers.len(), 5);
                for (index, pointer) in pointers.iter().enumerate() {
                    assert_eq!(pointer.buffer, index as u64 + 1);
                    assert_eq!(pointer.kernarg_offset, u32::try_from(index * 16).unwrap());
                    assert_eq!(pointer.buffer_offset, abi::GUARD as u64);
                    assert_eq!(pointer.extent_bytes, self.case.lengths()[index] * 4);
                    assert_eq!(pointer.access, abi::ACCESS[index]);
                }
                if self.mutation == 22 {
                    return Err("uncertain completion".into());
                }
                self.complete();
                if self.mutation == 16 {
                    ResponseV1::Written
                } else {
                    ResponseV1::Dispatched { elapsed_ns: 77 }
                }
            }
            CommandV1::Read {
                buffer,
                offset,
                bytes,
            } => {
                self.reads += 1;
                assert_eq!(self.dispatches, 1);
                assert_eq!(offset, 0);
                response_payload = self.buffers[&buffer].clone();
                assert_eq!(response_payload.len(), bytes as usize);
                if self.mutation == 20 {
                    response_payload.pop();
                }
                ResponseV1::Read {
                    payload_bytes: u32::try_from(response_payload.len()).unwrap(),
                }
            }
            CommandV1::Free { buffer } => {
                self.frees.push(buffer);
                assert!(self.buffers.remove(&buffer).is_some());
                if self.mutation == 17 {
                    ResponseV1::Written
                } else {
                    ResponseV1::Freed
                }
            }
            CommandV1::Close => {
                self.closed = true;
                if self.mutation == 18 {
                    ResponseV1::Written
                } else {
                    ResponseV1::Closed
                }
            }
            _ => panic!("unexpected publication command"),
        };
        Ok(Packet {
            response,
            payload: response_payload,
        })
    }
}
