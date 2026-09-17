use std::collections::BTreeMap;

use fe2o3_kfd::engineering_wire::{CommandV1, ResponseV1};

use crate::atomic_artifact as abi;
use crate::session::{Packet, Transport};
use crate::Result;

pub struct Mock {
    pub commands: u32,
    pub allocations: u64,
    pub dispatches: u32,
    pub frees: Vec<u64>,
    pub closed: bool,
    buffers: BTreeMap<u64, Vec<u8>>,
    mutation: u32,
}

impl Mock {
    pub fn new(mutation: u32) -> Self {
        Self {
            commands: 0,
            allocations: 0,
            dispatches: 0,
            frees: vec![],
            closed: false,
            buffers: BTreeMap::new(),
            mutation,
        }
    }

    fn complete(&mut self) {
        self.dispatches += 1;
        let range = abi::GUARD..abi::GUARD + abi::BYTES;
        let inputs = self.buffers[&2][range.clone()].to_vec();
        for (buffer, mask) in [(1, 0x5a), (3, 0xc3)] {
            let storage = self.buffers.get_mut(&buffer).unwrap();
            for (actual, expected) in storage[range.clone()].iter().zip(&inputs) {
                assert_eq!(*actual, *expected ^ mask);
                assert_ne!(actual, expected);
            }
            storage[range.clone()].copy_from_slice(&inputs);
        }
        if self.mutation == 5 {
            self.buffers.get_mut(&2).unwrap()[abi::GUARD] ^= 1;
        }
        if (6..=11).contains(&self.mutation) {
            let index = self.mutation - 6;
            let buffer = u64::from(index / 2 + 1);
            let offset = if index.is_multiple_of(2) {
                0
            } else {
                abi::GUARD + abi::BYTES
            };
            self.buffers.get_mut(&buffer).unwrap()[offset] ^= 1;
        }
        if matches!(self.mutation, 19 | 20) {
            let buffer = if self.mutation == 19 { 1 } else { 3 };
            self.buffers.get_mut(&buffer).unwrap()[abi::GUARD] ^= 1;
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
                    kernel: if self.mutation == 18 { 0 } else { 7 },
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
                assert_eq!(kernel, 7);
                assert_eq!(workgroup, abi::WORKGROUP);
                assert_eq!(grid, abi::GRID);
                assert_eq!(timeout_ms, 10_000);
                assert_eq!(payload, abi::kernarg(&super::metadata()).unwrap());
                assert_eq!(pointers.len(), 3);
                for (index, pointer) in pointers.iter().enumerate() {
                    assert_eq!(pointer.buffer, index as u64 + 1);
                    assert_eq!(pointer.kernarg_offset, u32::try_from(index * 16).unwrap());
                    assert_eq!(pointer.buffer_offset, abi::GUARD as u64);
                    assert_eq!(pointer.extent_bytes, abi::BYTES as u64);
                    assert_eq!(pointer.access, abi::ACCESS[index]);
                }
                self.complete();
                if self.mutation == 12 {
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
                assert_eq!(offset, 0);
                response_payload = self.buffers[&buffer].clone();
                if self.mutation == 13 {
                    response_payload.pop();
                }
                ResponseV1::Read {
                    payload_bytes: bytes + u32::from(self.mutation == 14),
                }
            }
            CommandV1::Free { buffer } => {
                self.buffers.remove(&buffer).unwrap();
                self.frees.push(buffer);
                if self.mutation == 15 {
                    ResponseV1::Written
                } else {
                    ResponseV1::Freed
                }
            }
            CommandV1::Close => {
                assert!(self.buffers.is_empty());
                self.closed = true;
                if self.mutation == 16 {
                    ResponseV1::Written
                } else {
                    ResponseV1::Closed
                }
            }
            _ => panic!("unexpected atomic channel command"),
        };
        if self.mutation == 17 && matches!(response, ResponseV1::Dispatched { .. }) {
            response_payload.push(0);
        }
        Ok(Packet {
            response,
            payload: response_payload,
        })
    }
}
