//! Explicit unauthenticated engineering entry, outside the safe Ferric adapter.
//! Every error terminates this disposable process; no native retry is permitted.

#[allow(dead_code)] // The same module is also compiled by the safe parent.
mod wire;

use fe2o3_kfd::engineering_wire::{CommandV1, PointerFixupV1, ResponseV1};
use fe2o3_kfd::{
    Gfx950EngineeringPeerBufferV1 as Buffer, Gfx950EngineeringPeerDispatchV1 as Dispatch,
    Gfx950EngineeringPeerGroupV1 as Group, Gfx950EngineeringPeerKernelV1 as Kernel,
    Gfx950EngineeringPeerPointerV1 as Pointer,
};
use std::collections::{BTreeMap, BTreeSet};

type Result<T> = std::result::Result<T, String>;

fn resolve_pointers(
    buffers: &BTreeMap<u64, Buffer>,
    pointers: &[PointerFixupV1],
) -> Result<Vec<Pointer>> {
    pointers
        .iter()
        .map(|pointer| {
            buffers
                .get(&pointer.buffer)
                .map(|buffer| {
                    buffer.pointer(
                        pointer.kernarg_offset,
                        pointer.buffer_offset,
                        pointer.extent_bytes,
                        pointer.access,
                    )
                })
                .ok_or_else(|| "peer dispatch references an unknown buffer".into())
        })
        .collect()
}

struct Worker {
    group: Group,
    buffers: BTreeMap<u64, Buffer>,
    kernels: BTreeMap<u64, Kernel>,
    next_buffer: u64,
    next_kernel: u64,
    next_request: u64,
    world: usize,
    concurrent_rounds: bool,
}

impl Worker {
    fn owned(&self, rank: usize, id: u64) -> Result<Buffer> {
        self.buffers
            .get(&id)
            .copied()
            .filter(|buffer| buffer.owner_rank() == rank)
            .ok_or_else(|| "peer host access requires an owned buffer".into())
    }

    fn execute(
        &mut self,
        request: wire::Request,
        payload: Vec<u8>,
    ) -> Result<(ResponseV1, Vec<u8>)> {
        if request.id != self.next_request || request.rank as usize >= self.world {
            return Err("peer request identity or rank drift".into());
        }
        request.payload_bytes().map_err(|error| error.to_string())?;
        let round = !request.round_ranks.is_empty();
        if round
            && (!self.concurrent_rounds
                || request
                    .round_ranks
                    .iter()
                    .any(|&rank| rank as usize >= self.world))
            || self.concurrent_rounds
                && !round
                && matches!(
                    request.command,
                    CommandV1::Dispatch { .. } | CommandV1::DispatchSequence { .. }
                )
        {
            return Err("dispatch does not match the explicit peer execution profile".into());
        }
        self.next_request = self
            .next_request
            .checked_add(1)
            .ok_or("request identity exhausted")?;
        let rank = request.rank as usize;
        let response = match request.command {
            CommandV1::ConfigurePerformance {
                cache_kernel_admission,
                operational_currentness,
                profile,
            } if rank == 0 && !profile => {
                self.group
                    .configure_performance(cache_kernel_admission, operational_currentness)?;
                ResponseV1::PerformanceConfigured
            }
            CommandV1::Allocate { bytes } => {
                let peers = (0..self.world)
                    .filter(|&peer| request.peer_readable && peer != rank)
                    .collect::<Vec<_>>();
                let token = self.group.allocate(rank, &peers, bytes)?;
                let id = self.next_buffer;
                self.next_buffer = id.checked_add(1).ok_or("buffer identity exhausted")?;
                self.buffers.insert(id, token);
                ResponseV1::Allocated { buffer: id, bytes }
            }
            CommandV1::Free { buffer } => {
                let token = self.owned(rank, buffer)?;
                self.group.release(token)?;
                self.buffers.remove(&buffer);
                ResponseV1::Freed
            }
            CommandV1::Write { buffer, offset, .. } => {
                self.group
                    .write(self.owned(rank, buffer)?, offset, &payload)?;
                ResponseV1::Written
            }
            CommandV1::Read {
                buffer,
                offset,
                bytes,
            } => {
                let payload = self.group.read(self.owned(rank, buffer)?, offset, bytes)?;
                return Ok((
                    ResponseV1::Read {
                        payload_bytes: bytes,
                    },
                    payload,
                ));
            }
            CommandV1::LoadKernel {
                object_sha256,
                symbol,
                ..
            } => {
                let kernel = self
                    .group
                    .load_kernel(rank, payload, object_sha256, symbol)?;
                let id = self.next_kernel;
                self.next_kernel = id.checked_add(1).ok_or("kernel identity exhausted")?;
                let metadata = kernel.metadata().clone();
                self.kernels.insert(id, kernel);
                ResponseV1::LoadedKernel {
                    kernel: id,
                    metadata,
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
                let kernel = self
                    .kernels
                    .get(&kernel)
                    .filter(|kernel| kernel.rank() == rank)
                    .ok_or("peer kernel rank or identity mismatch")?;
                let pointers = resolve_pointers(&self.buffers, &pointers)?;
                // SAFETY: the explicit CLI opt-in authorizes unauthenticated code.
                // This single-threaded disposable process owns the entire group;
                // the runtime checks each token, extent, access and completion.
                let elapsed_ns = unsafe {
                    self.group
                        .dispatch_unchecked(kernel, payload, workgroup, grid, &pointers, timeout_ms)
                }?;
                ResponseV1::Dispatched { elapsed_ns }
            }
            CommandV1::DispatchSequence { dispatches } => {
                let mut offset = 0_usize;
                let commands = dispatches
                    .into_iter()
                    .enumerate()
                    .map(|(index, dispatch)| {
                        let rank = if round {
                            request.round_ranks[index] as usize
                        } else {
                            rank
                        };
                        let kernel = self
                            .kernels
                            .get(&dispatch.kernel)
                            .filter(|kernel| kernel.rank() == rank)
                            .ok_or("peer sequence kernel rank or identity mismatch")?;
                        let end = offset
                            .checked_add(dispatch.payload_bytes as usize)
                            .ok_or("peer sequence payload overflow")?;
                        let bytes = payload
                            .get(offset..end)
                            .ok_or("peer sequence payload truncated")?
                            .to_vec();
                        offset = end;
                        Ok(Dispatch {
                            kernel,
                            bytes,
                            workgroup: dispatch.workgroup,
                            grid: dispatch.grid,
                            pointers: resolve_pointers(&self.buffers, &dispatch.pointers)?,
                            timeout_ms: dispatch.timeout_ms,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                if offset != payload.len() {
                    return Err("peer sequence payload trailing bytes".into());
                }
                // SAFETY: the mandatory process opt-in covers each retained kernel;
                // the group prevalidates the whole bounded sequence before publishing
                // and observes every completion with all participant owners retained.
                let elapsed_ns = unsafe {
                    if round {
                        self.group.dispatch_round_unchecked(commands)
                    } else {
                        self.group.dispatch_sequence_unchecked(commands)
                    }
                }?;
                ResponseV1::DispatchSequenceCompleted { elapsed_ns }
            }
            CommandV1::Close if rank == 0 => {
                self.group.close()?;
                ResponseV1::Closed
            }
            _ => return Err("command is unsupported by the serial peer profile".into()),
        };
        Ok((response, vec![]))
    }
}

fn parse_args(args: &[String]) -> Result<Vec<u64>> {
    if args.len() != 3
        || args[0] != "--allow-unauthenticated-machine-code"
        || args[1] != "--device-unique-ids"
    {
        return Err(
            "requires --allow-unauthenticated-machine-code --device-unique-ids UID,UID[,six more]"
                .into(),
        );
    }
    let ids = args[2]
        .split(',')
        .map(|value| value.parse::<u64>().map_err(|_| "invalid unique ID".into()))
        .collect::<Result<Vec<_>>>()?;
    if !matches!(ids.len(), 2 | 8)
        || ids.contains(&0)
        || ids.iter().copied().collect::<BTreeSet<_>>().len() != ids.len()
    {
        return Err("peer roster requires exactly two or eight distinct nonzero IDs".into());
    }
    Ok(ids)
}

fn run() -> Result<()> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    let concurrent_rounds = args.last().is_some_and(|arg| arg == "--concurrent-rounds");
    if concurrent_rounds {
        args.pop();
    }
    let ids = parse_args(&args)?;
    // SAFETY: this dedicated entry has no threads or independent GPU clients.
    // Its mandatory opt-in is checked before opening KFD. Any error exits main.
    let group = unsafe { Group::open_unchecked(&ids) }?;
    let mut worker = Worker {
        group,
        buffers: BTreeMap::new(),
        kernels: BTreeMap::new(),
        next_buffer: 1,
        next_kernel: 1,
        next_request: 1,
        world: ids.len(),
        concurrent_rounds,
    };
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    wire::write_response(
        &mut output,
        &wire::Response::Ready {
            protocol: wire::PROTOCOL,
            mode: if concurrent_rounds {
                wire::ROUND_MODE
            } else {
                wire::MODE
            }
            .into(),
            target: "gfx950:xnack-".into(),
            unique_ids: ids,
            process_id: std::process::id(),
            authority: "none".into(),
        },
        &[],
    )
    .map_err(|e| e.to_string())?;
    loop {
        let (request, payload) = wire::read_request(&mut input)
            .map_err(|e| e.to_string())?
            .ok_or("peer controller closed without group teardown")?;
        let id = request.id;
        let rank = request.rank;
        let round_ranks = request.round_ranks.clone();
        let (response, payload) = match worker.execute(request, payload) {
            Ok(response) => response,
            Err(message) => {
                let _ = wire::write_response(
                    &mut output,
                    &completion(
                        id,
                        rank,
                        round_ranks,
                        ResponseV1::Error {
                            message: message.clone(),
                            fatal: true,
                        },
                    ),
                    &[],
                );
                return Err(message);
            }
        };
        let closed = matches!(response, ResponseV1::Closed);
        wire::write_response(
            &mut output,
            &completion(id, rank, round_ranks, response),
            &payload,
        )
        .map_err(|e| e.to_string())?;
        if closed {
            return Ok(());
        }
    }
}

fn completion(request: u64, rank: u32, ranks: Vec<u32>, response: ResponseV1) -> wire::Response {
    if ranks.is_empty() {
        wire::Response::Done {
            request,
            rank,
            response,
        }
    } else {
        wire::Response::RoundDone {
            request,
            ranks,
            response,
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("ferric engineering peer worker: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mandatory_opt_in_and_exact_device_roster_precede_gpu_open() {
        let args = |ids: &str| {
            vec![
                "--allow-unauthenticated-machine-code".into(),
                "--device-unique-ids".into(),
                ids.into(),
            ]
        };
        assert_eq!(parse_args(&args("1,2")).unwrap(), [1, 2]);
        for ids in ["", "1", "1,1", "0,2", "1,2,3", "1,no", "1,2,"] {
            assert!(parse_args(&args(ids)).is_err());
        }
        assert!(parse_args(&["--device-unique-ids".into(), "1,2".into()]).is_err());
    }
}
