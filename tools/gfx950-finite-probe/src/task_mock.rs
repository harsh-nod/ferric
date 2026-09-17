use std::collections::BTreeMap;

use fe2o3_kfd::engineering_wire::{CommandV1, ResponseV1};

use crate::session::{Packet, Transport};
use crate::task_artifact as abi;
use crate::task_probe;
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
        let expected = self.dispatches;
        assert_eq!(
            &self.buffers[&2][abi::GUARD..abi::GUARD + 4],
            &expected.to_le_bytes()
        );
        let initial = task_probe::initial_state(expected.min(4));
        for (index, value) in initial.iter().enumerate() {
            assert_eq!(
                &self.buffers[&(index as u64 + 3)][abi::GUARD..abi::GUARD + 4],
                &value.to_le_bytes()
            );
        }
        let mut state = initial;
        if expected == 5 {
            state[5] = 1;
            if self.mutation == 9 {
                state[6] = 1;
            }
        } else {
            state[1] = 0;
            state[2] = 127;
            state[3] = 127;
            state[4] = 0x1555;
            // Transport tests intentionally do not substitute for a numerical reference.
            state[6..].fill(42);
        }
        match self.mutation {
            6 => state[4] |= 1 << 14,
            7 => state[2] ^= 1,
            8 => state[0] += 1,
            _ => {}
        }
        for (index, value) in state.iter().enumerate() {
            self.buffers.get_mut(&(index as u64 + 3)).unwrap()[abi::GUARD..abi::GUARD + 4]
                .copy_from_slice(&value.to_le_bytes());
        }
        if self.mutation == 4 {
            self.buffers.get_mut(&1).unwrap()[abi::GUARD] ^= 1;
        }
        if self.mutation == 5 {
            self.buffers.get_mut(&15).unwrap()[0] ^= 1;
        }
        if self.mutation == 12 {
            self.buffers.get_mut(&2).unwrap()[abi::GUARD] ^= 1;
        }
    }
}

impl Transport for Mock {
    fn request(&mut self, command: CommandV1, payload: Vec<u8>) -> Result<Packet> {
        self.commands += 1;
        assert_eq!(command.payload_bytes().unwrap(), payload.len());
        let mut response_payload = vec![];
        let response = match command {
            CommandV1::LoadKernel { .. } => {
                let mut metadata = super::metadata();
                if self.mutation == 1 {
                    metadata.object_sha256[0] ^= 1;
                }
                ResponseV1::LoadedKernel {
                    kernel: 7,
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
                if self.mutation == 15 {
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
                assert_eq!(workgroup, [128, 1, 1]);
                assert_eq!(grid, [256, 1, 1]);
                assert_eq!(timeout_ms, 10_000);
                assert_eq!(payload, abi::kernarg(&super::metadata()).unwrap());
                assert_eq!(pointers.len(), 15);
                for (index, pointer) in pointers.iter().enumerate() {
                    assert_eq!(pointer.buffer, index as u64 + 1);
                    assert_eq!(pointer.kernarg_offset, abi::pointer_offset(index));
                    assert_eq!(pointer.buffer_offset, 64);
                    assert_eq!(pointer.extent_bytes, abi::buffer_bytes(index) as u64);
                    assert_eq!(pointer.access, abi::buffer_access(index));
                }
                self.complete();
                if self.mutation == 13 {
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
                if self.mutation == 14 {
                    response_payload.pop();
                }
                ResponseV1::Read {
                    payload_bytes: bytes,
                }
            }
            CommandV1::Free { buffer } => {
                self.buffers.remove(&buffer).unwrap();
                self.frees.push(buffer);
                if self.mutation == 10 {
                    ResponseV1::Written
                } else {
                    ResponseV1::Freed
                }
            }
            CommandV1::Close => {
                assert!(self.buffers.is_empty());
                self.closed = true;
                if self.mutation == 11 {
                    ResponseV1::Written
                } else {
                    ResponseV1::Closed
                }
            }
            _ => panic!("unexpected task command"),
        };
        Ok(Packet {
            response,
            payload: response_payload,
        })
    }
}
