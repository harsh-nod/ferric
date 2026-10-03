//! One transport type for independent and explicitly named peer profiles.

use super::{tp_peer_worker::PeerWorker, tp_prepared_worker::PreparedWorker, tp_worker::Worker};
use ferric_m1_engineering_execution_v1::tp_execution::{
    EngineeringTp2CollectiveReceiptV1, EngineeringTp2CollectiveRequestV1,
    EngineeringTp2GraphGeometryV1, EngineeringTp2GraphInputV1, EngineeringTp2GraphKernelProfileV1,
    EngineeringTp2GraphPolicyV1, EngineeringTp2PreparedGraphReceiptV1,
    EngineeringTp2PreparedInputV1, EngineeringTp2PreparedProgramV1,
    EngineeringTp2PreparedReceiptV1, EngineeringTpDispatchV1, EngineeringTpRankTransportV1,
    TpResult,
};

pub enum RankWorker {
    Independent(Worker),
    Peer(PeerWorker),
    PreparedPeer(PreparedWorker),
}

impl RankWorker {
    fn transport(&self) -> &dyn EngineeringTpRankTransportV1 {
        match self {
            Self::Independent(worker) => worker,
            Self::Peer(worker) => worker,
            Self::PreparedPeer(worker) => worker,
        }
    }
    fn transport_mut(&mut self) -> &mut dyn EngineeringTpRankTransportV1 {
        match self {
            Self::Independent(worker) => worker,
            Self::Peer(worker) => worker,
            Self::PreparedPeer(worker) => worker,
        }
    }
    pub fn pid(&self) -> u32 {
        match self {
            Self::Independent(worker) => worker.pid(),
            Self::Peer(worker) => worker.pid(),
            Self::PreparedPeer(worker) => worker.pid(),
        }
    }
}

impl EngineeringTpRankTransportV1 for RankWorker {
    fn setup_upload_chunk_bytes(&self) -> usize {
        self.transport().setup_upload_chunk_bytes()
    }
    fn supports_prepared_peer_graph(&self, policy: EngineeringTp2GraphPolicyV1) -> bool {
        self.transport().supports_prepared_peer_graph(policy)
    }
    fn supports_prepared_peer_graph_profile(
        &self,
        policy: EngineeringTp2GraphPolicyV1,
        profile: EngineeringTp2GraphKernelProfileV1,
    ) -> bool {
        self.transport()
            .supports_prepared_peer_graph_profile(policy, profile)
    }
    fn register_prepared_peer_graph(
        &mut self,
        program: &EngineeringTp2PreparedProgramV1,
        policy: EngineeringTp2GraphPolicyV1,
    ) -> TpResult<[u8; 32]> {
        self.transport_mut()
            .register_prepared_peer_graph(program, policy)
    }
    fn execute_prepared_peer_graph(
        &mut self,
        input: &EngineeringTp2PreparedInputV1,
    ) -> TpResult<EngineeringTp2PreparedGraphReceiptV1> {
        self.transport_mut().execute_prepared_peer_graph(input)
    }
    fn supports_prepared_peer_graph_geometry(
        &self,
        policy: EngineeringTp2GraphPolicyV1,
        profile: EngineeringTp2GraphKernelProfileV1,
        geometry: EngineeringTp2GraphGeometryV1,
    ) -> bool {
        self.transport()
            .supports_prepared_peer_graph_geometry(policy, profile, geometry)
    }
    fn register_prepared_peer_graph_geometry(
        &mut self,
        program: &EngineeringTp2PreparedProgramV1,
        policy: EngineeringTp2GraphPolicyV1,
        geometry: EngineeringTp2GraphGeometryV1,
    ) -> TpResult<[u8; 32]> {
        self.transport_mut()
            .register_prepared_peer_graph_geometry(program, policy, geometry)
    }
    fn execute_prepared_peer_graph_geometry(
        &mut self,
        input: &EngineeringTp2GraphInputV1,
    ) -> TpResult<EngineeringTp2PreparedGraphReceiptV1> {
        self.transport_mut()
            .execute_prepared_peer_graph_geometry(input)
    }
    fn supports_prepared_peer(&self) -> bool {
        self.transport().supports_prepared_peer()
    }
    fn register_prepared_peer(
        &mut self,
        program: &EngineeringTp2PreparedProgramV1,
    ) -> TpResult<[u8; 32]> {
        self.transport_mut().register_prepared_peer(program)
    }
    fn execute_prepared_peer(
        &mut self,
        input: &EngineeringTp2PreparedInputV1,
    ) -> TpResult<EngineeringTp2PreparedReceiptV1> {
        self.transport_mut().execute_prepared_peer(input)
    }
    fn supports_peer_dependency_collectives(&self) -> bool {
        self.transport().supports_peer_dependency_collectives()
    }
    fn execute_peer_dependency_collective(
        &mut self,
        request: &EngineeringTp2CollectiveRequestV1,
    ) -> TpResult<EngineeringTp2CollectiveReceiptV1> {
        self.transport_mut()
            .execute_peer_dependency_collective(request)
    }
    fn require_loaded_image(&mut self, image: [u8; 32], kernels: &[&str]) -> TpResult<()> {
        self.transport_mut().require_loaded_image(image, kernels)
    }
    fn runtime_diagnostic_snapshot(&mut self) -> TpResult<serde_json::Value> {
        self.transport_mut().runtime_diagnostic_snapshot()
    }
    fn supports_concurrent_rounds(&self) -> bool {
        self.transport().supports_concurrent_rounds()
    }
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
    fn supports_ordered_batches(&self) -> bool {
        self.transport().supports_ordered_batches()
    }
    fn submit_ordered_batch(&mut self, dispatches: &[EngineeringTpDispatchV1]) -> TpResult<()> {
        self.transport_mut().submit_ordered_batch(dispatches)
    }
    fn wait_ordered_batch(&mut self, count: usize) -> TpResult<()> {
        self.transport_mut().wait_ordered_batch(count)
    }
    fn supports_full_forward(&self) -> bool {
        self.transport().supports_full_forward()
    }
    fn submit_full_forward(&mut self, dispatches: &[EngineeringTpDispatchV1]) -> TpResult<()> {
        self.transport_mut().submit_full_forward(dispatches)
    }
    fn wait_full_forward(&mut self, count: usize) -> TpResult<()> {
        self.transport_mut().wait_full_forward(count)
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

#[cfg(test)]
mod tests {
    use super::super::tp_worker::tests::full_forward_wrapper_fixture;
    use super::*;

    #[test]
    fn full_forward_wrapper_preserves_enabled_and_disabled_capabilities() {
        for enabled in [false, true] {
            let mut worker = RankWorker::Independent(full_forward_wrapper_fixture(enabled, None));
            assert_eq!(worker.supports_full_forward(), enabled);
            assert!(!worker.supports_peer_dependency_collectives());
            assert!(!worker.supports_prepared_peer());
            // The wrapper must preserve the inner rejection, including closure.
            assert!(worker.submit_full_forward(&[]).is_err());
            assert!(worker.allocate(4).is_err());
            worker.close().unwrap();
        }
    }

    #[test]
    fn full_forward_wrapper_keeps_peer_transport_unsupported() {
        let peers = super::super::tp_peer_worker::tests::fixture("normal");
        for peer in peers {
            let mut worker = RankWorker::Peer(peer);
            assert!(!worker.supports_full_forward());
            assert!(!worker.supports_peer_dependency_collectives());
            assert!(!worker.supports_prepared_peer());
            assert!(worker.submit_full_forward(&[]).is_err());
            assert!(worker.wait_full_forward(616).is_err());
            worker.close().unwrap();
        }
    }

    #[test]
    #[ignore = "requires FERRIC_V8_TEST_ARTIFACT; fake IPC with admitted metadata, no GPU"]
    fn actual_v8_metadata_full_forward_pack_through_rank_wrapper() {
        use ferric_m1_engineering_execution_v1::tp_artifact::EngineeringTpArtifactV1;
        use ferric_m1_engineering_execution_v1::tp_execution::{
            EngineeringTpArgumentV1, EngineeringTpBufferAccessV1,
        };
        let path = std::env::var_os("FERRIC_V8_TEST_ARTIFACT").expect("explicit v8 artifact path");
        let artifact = EngineeringTpArtifactV1::open_fp32_head32(
            std::path::Path::new(&path),
            &ferric_qwen3_tp_fp32_head32_kernels_device_v8::compiler_expectation_roster_v8(),
        )
        .unwrap();
        let mut worker =
            RankWorker::Independent(full_forward_wrapper_fixture(true, Some(&artifact)));
        assert!(worker.supports_full_forward());
        let logits = worker.allocate(32 * 151_936 * 4).unwrap();
        let choice = worker.allocate(32 * 4).unwrap();
        let dispatch = EngineeringTpDispatchV1 {
            kernel: "ferric_qwen3_tp_batch32_argmax_f32_v8",
            grid_workgroups: 1,
            workgroup_size: 64,
            arguments: vec![
                EngineeringTpArgumentV1::Buffer {
                    id: logits,
                    offset: 0,
                    elements: 32 * 151_936,
                    element_bytes: 4,
                    access: EngineeringTpBufferAccessV1::Read,
                },
                EngineeringTpArgumentV1::Buffer {
                    id: choice,
                    offset: 0,
                    elements: 32,
                    element_bytes: 4,
                    access: EngineeringTpBufferAccessV1::Write,
                },
                EngineeringTpArgumentV1::U32(1),
            ],
        };
        for _ in 0..2 {
            worker
                .submit_full_forward(&vec![dispatch.clone(); 616])
                .unwrap();
            worker.wait_full_forward(616).unwrap();
            worker.read(choice, 0, &mut [0; 4]).unwrap();
        }
        worker.close().unwrap();
    }
}
