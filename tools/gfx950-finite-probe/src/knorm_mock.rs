use crate::{
    knorm_artifact as abi, kproj_artifact,
    session::{Packet, Transport},
    Result,
};
use fe2o3_kfd::engineering_wire::{BufferAccessV1, CommandV1, ResponseV1};
use std::collections::BTreeMap;

pub struct Mock {
    pub commands: u32,
    pub loads: u32,
    pub allocations: u64,
    pub writes: u32,
    pub dispatches: u32,
    pub frees: Vec<u64>,
    pub closed: bool,
    pub intermediate_read: bool,
    buffers: BTreeMap<u64, Vec<u8>>,
    mutation: u32,
}

impl Mock {
    pub fn new(mutation: u32) -> Self {
        Self {
            commands: 0,
            loads: 0,
            allocations: 0,
            writes: 0,
            dispatches: 0,
            frees: vec![],
            closed: false,
            intermediate_read: false,
            buffers: BTreeMap::new(),
            mutation,
        }
    }

    fn complete(&mut self, consumer: bool) {
        self.dispatches += 1;
        if consumer {
            for (buffer, bits) in [(5, 0x3f80_u16), (6, 0x4000)] {
                self.buffers.get_mut(&buffer).unwrap()[abi::GUARD..abi::GUARD + 2048]
                    .copy_from_slice(&bits.to_le_bytes().repeat(1024));
            }
            if self.mutation == 10 {
                self.buffers.get_mut(&3).unwrap()[abi::GUARD] ^= 1;
            }
            for (mutation, buffer) in [(11, 1), (12, 4), (13, 2)] {
                if self.mutation == mutation {
                    self.buffers.get_mut(&buffer).unwrap()[abi::GUARD] ^= 1;
                }
            }
            for (mutation, buffer) in [(26, 5), (27, 6)] {
                if self.mutation == mutation {
                    self.buffers.get_mut(&buffer).unwrap()[abi::GUARD..abi::GUARD + 2]
                        .copy_from_slice(&0x7fc0_u16.to_le_bytes());
                }
            }
        } else {
            let bits = if self.mutation == 9 {
                0x7fc0_1234
            } else {
                1.0_f32.to_bits()
            };
            self.buffers.get_mut(&3).unwrap()[abi::GUARD..abi::GUARD + 4096]
                .copy_from_slice(&bits.to_le_bytes().repeat(1024));
        }
        if (14..=25).contains(&self.mutation) {
            let index = (self.mutation - 14) as usize;
            let buffer = index / 2;
            if consumer || buffer == 2 {
                let offset = if index.is_multiple_of(2) {
                    0
                } else {
                    abi::GUARD + abi::BYTES[buffer]
                };
                self.buffers.get_mut(&(buffer as u64 + 1)).unwrap()[offset] ^= 1;
            }
        }
    }
}

impl Transport for Mock {
    fn request(&mut self, command: CommandV1, payload: Vec<u8>) -> Result<Packet> {
        self.commands += 1;
        assert_eq!(command.payload_bytes().unwrap(), payload.len());
        let mut output = vec![];
        let response = match command {
            CommandV1::LoadKernel { symbol, .. } => {
                self.loads += 1;
                let consumer = self.loads == 2;
                assert_eq!(
                    symbol,
                    if consumer {
                        abi::SYMBOL
                    } else {
                        kproj_artifact::WAVE64_SYMBOL
                    }
                );
                let mut metadata = super::metadata(consumer);
                if self.mutation == self.loads {
                    metadata.object_sha256[0] ^= 1;
                }
                ResponseV1::LoadedKernel {
                    kernel: if self.mutation == 3 {
                        1
                    } else {
                        u64::from(self.loads)
                    },
                    metadata,
                }
            }
            CommandV1::Allocate { bytes } => {
                self.allocations += 1;
                let buffer = if self.mutation == 4 {
                    1
                } else {
                    self.allocations
                };
                self.buffers
                    .insert(buffer, vec![0; usize::try_from(bytes).unwrap()]);
                ResponseV1::Allocated {
                    buffer,
                    bytes: bytes + u64::from(self.mutation == 5),
                }
            }
            CommandV1::Write { buffer, offset, .. } => {
                assert_eq!(self.dispatches, 0, "no host intermediate reupload");
                assert_eq!(offset, 0);
                self.writes += 1;
                assert_eq!(payload.len(), self.buffers[&buffer].len());
                *self.buffers.get_mut(&buffer).unwrap() = payload;
                if self.mutation == 6 {
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
                let consumer = self.dispatches == 1;
                assert_eq!(kernel, if consumer { 2 } else { 1 });
                assert_eq!(workgroup, abi::WORKGROUP);
                assert_eq!(
                    grid,
                    if consumer {
                        abi::GRID
                    } else {
                        kproj_artifact::WAVE64_GRID
                    }
                );
                assert_eq!(timeout_ms, 10_000);
                assert_eq!(
                    payload,
                    if consumer {
                        abi::kernarg(&super::metadata(true)).unwrap()
                    } else {
                        kproj_artifact::kernarg(&super::metadata(false)).unwrap()
                    }
                );
                let indices: &[usize] = if consumer { &[2, 3, 4, 5] } else { &[0, 1, 2] };
                assert_eq!(pointers.len(), indices.len());
                for (argument, (pointer, index)) in pointers.iter().zip(indices).enumerate() {
                    assert_eq!(pointer.buffer, *index as u64 + 1);
                    assert_eq!(
                        pointer.kernarg_offset,
                        u32::try_from(argument * 16).unwrap()
                    );
                    assert_eq!(pointer.buffer_offset, abi::GUARD as u64);
                    assert_eq!(pointer.extent_bytes, abi::BYTES[*index] as u64);
                    assert_eq!(
                        pointer.access,
                        if argument < 2 {
                            BufferAccessV1::Read
                        } else {
                            BufferAccessV1::Write
                        }
                    );
                }
                if consumer {
                    assert!(self.intermediate_read);
                }
                self.complete(consumer);
                if self.mutation == if consumer { 8 } else { 7 } {
                    return Err("simulated bounded dispatch failure".into());
                }
                if self.mutation == 31 {
                    output.push(0);
                }
                ResponseV1::Dispatched {
                    elapsed_ns: 70 + u64::from(self.dispatches),
                }
            }
            CommandV1::Read {
                buffer,
                offset,
                bytes,
            } => {
                assert_eq!(offset, 0);
                if self.dispatches == 1 && buffer == 3 {
                    self.intermediate_read = true;
                }
                output = self.buffers[&buffer].clone();
                if self.mutation == 28 {
                    output.pop();
                }
                ResponseV1::Read {
                    payload_bytes: bytes,
                }
            }
            CommandV1::Free { buffer } => {
                self.buffers.remove(&buffer).unwrap();
                self.frees.push(buffer);
                if self.mutation == 29 {
                    ResponseV1::Written
                } else {
                    ResponseV1::Freed
                }
            }
            CommandV1::Close => {
                assert!(self.buffers.is_empty());
                self.closed = true;
                if self.mutation == 30 {
                    ResponseV1::Written
                } else {
                    ResponseV1::Closed
                }
            }
            _ => panic!("unexpected chain command"),
        };
        Ok(Packet {
            response,
            payload: output,
        })
    }
}
