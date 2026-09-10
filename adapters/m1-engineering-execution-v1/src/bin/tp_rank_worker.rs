//! One transport type for the independent and explicitly serial peer profiles.

use super::{tp_peer_worker::PeerWorker, tp_worker::Worker};
use ferric_m1_engineering_execution_v1::tp_execution::{
    EngineeringTpDispatchV1, EngineeringTpRankTransportV1, TpResult,
};

pub enum RankWorker {
    Independent(Worker),
    Peer(PeerWorker),
}

impl RankWorker {
    fn transport(&self) -> &dyn EngineeringTpRankTransportV1 {
        match self {
            Self::Independent(worker) => worker,
            Self::Peer(worker) => worker,
        }
    }
    fn transport_mut(&mut self) -> &mut dyn EngineeringTpRankTransportV1 {
        match self {
            Self::Independent(worker) => worker,
            Self::Peer(worker) => worker,
        }
    }
    pub fn pid(&self) -> u32 {
        match self {
            Self::Independent(worker) => worker.pid(),
            Self::Peer(worker) => worker.pid(),
        }
    }
}

impl EngineeringTpRankTransportV1 for RankWorker {
    fn peer_group_rank(&self) -> Option<(u32, u32, u32)> {
        self.transport().peer_group_rank()
    }
    fn allocate(&mut self, bytes: usize) -> TpResult<u64> {
        self.transport_mut().allocate(bytes)
    }
    fn allocate_peer_readable(&mut self, bytes: usize) -> TpResult<u64> {
        self.transport_mut().allocate_peer_readable(bytes)
    }
    fn write(&mut self, buffer: u64, offset: usize, bytes: &[u8]) -> TpResult<()> {
        self.transport_mut().write(buffer, offset, bytes)
    }
    fn read(&mut self, buffer: u64, offset: usize, bytes: &mut [u8]) -> TpResult<()> {
        self.transport_mut().read(buffer, offset, bytes)
    }
    fn submit(&mut self, dispatch: &EngineeringTpDispatchV1) -> TpResult<()> {
        self.transport_mut().submit(dispatch)
    }
    fn wait(&mut self) -> TpResult<()> {
        self.transport_mut().wait()
    }
    fn supports_sequences(&self) -> bool {
        self.transport().supports_sequences()
    }
    fn submit_sequence(&mut self, dispatches: &[EngineeringTpDispatchV1]) -> TpResult<()> {
        self.transport_mut().submit_sequence(dispatches)
    }
    fn wait_sequence(&mut self, count: usize) -> TpResult<()> {
        self.transport_mut().wait_sequence(count)
    }
    fn supports_queue_rollover(&self) -> bool {
        self.transport().supports_queue_rollover()
    }
    fn prepare_packets(&mut self, count: u64) -> TpResult<()> {
        self.transport_mut().prepare_packets(count)
    }
    fn close(&mut self) -> TpResult<()> {
        self.transport_mut().close()
    }
}
