//! Checked physical-device acquisition and model-memory ownership for M1.
//!
//! This module is the only production bridge from fe2's checked gfx942 device
//! token into Ferric's device receipt. It snapshots canonical, redacted
//! observations and then consumes that same token into the service allocation
//! session. No API accepts a caller-authored device label or exposes the raw
//! session after this join.

use core::fmt;

use fe2o3_kfd::CheckedGfx942XnackMinusDevice;
use fe2o3_service_host::{
    ServiceAllocationAcquireErrorV1, ServiceAllocationReleaseFailureV1,
    ServiceAllocationReleaseObservationV1, ServiceAllocationSessionV1,
};
use ferric_build::AddresslessModelMemoryPlan;
use ferric_spec::Identity;
use sha2::{Digest, Sha256};

use crate::{
    allocate_initialized_model_memory_v1, BoundModelMemoryAllocationsV1, Gfx942DeviceBinding,
    InitializedModelMemoryAllocationErrorV1, InitializedModelMemoryAllocationFailureV1,
};

const M1_PHYSICAL_DEVICE_RECEIPT_DOMAIN_V1: &[u8] = b"ferric.m1.physical-device-receipt.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PhysicalDeviceReceiptComponentsV1 {
    admission_domain: [u8; 32],
    admission_profile: [u8; 32],
    physical_device_id: u64,
    admission_generation: u64,
    observation_epoch: u64,
    topology_node_id: u32,
    kfd_gpu_id: u32,
    gpu_unique_id: u64,
    pci: [u8; 5],
    render_descriptor: [u64; 3],
    render_numbers: [u32; 3],
    drm_version: [i32; 3],
    drm_identity: [u32; 7],
    aperture_gpu_id: u32,
    aperture: [u64; 6],
    process: [u64; 4],
}

fn checked_receipt_components(
    checked: &CheckedGfx942XnackMinusDevice,
) -> PhysicalDeviceReceiptComponentsV1 {
    let admission = checked.model_admission();
    let correlation = admission.correlation();
    let observation = checked.observation();
    let physical = correlation.identity();
    let pci = observation.pci();
    let render = observation.render_descriptor();
    let drm = observation.drm();
    let drm_device = drm.device();
    let drm_version = drm.driver_version();
    let aperture = observation.aperture();
    let process = checked.process_incarnation();

    debug_assert_eq!(
        correlation.topology_node_id(),
        observation.topology_node_id()
    );
    debug_assert_eq!(correlation.kfd_gpu_id(), observation.kfd_gpu_id());
    debug_assert_eq!(physical.gpu_unique_id, observation.unique_id());
    debug_assert_eq!(aperture.gpu_id(), observation.kfd_gpu_id());

    PhysicalDeviceReceiptComponentsV1 {
        admission_domain: *admission.domain_id().digest().as_bytes(),
        admission_profile: *correlation.profile_id().digest().as_bytes(),
        physical_device_id: admission.model_key().physical.0,
        admission_generation: admission.model_key().generation.0,
        observation_epoch: correlation.epoch().0,
        topology_node_id: observation.topology_node_id(),
        kfd_gpu_id: observation.kfd_gpu_id(),
        gpu_unique_id: observation.unique_id(),
        pci: [
            pci.domain().to_le_bytes()[0],
            pci.domain().to_le_bytes()[1],
            pci.bus(),
            pci.device(),
            pci.function(),
        ],
        render_descriptor: [
            render.file_system_device(),
            render.inode(),
            render.character_device(),
        ],
        render_numbers: [
            render.major(),
            render.minor(),
            u32::from(observation.render_minor()),
        ],
        drm_version: [drm_version.major, drm_version.minor, drm_version.patch],
        drm_identity: [
            drm.acceleration_working(),
            drm_device.device_id,
            drm_device.chip_rev,
            drm_device.external_rev,
            drm_device.pci_rev,
            drm_device.family,
            drm.vram_lost_counter(),
        ],
        aperture_gpu_id: aperture.gpu_id(),
        aperture: [
            aperture.lds().base(),
            aperture.lds().limit(),
            aperture.scratch().base(),
            aperture.scratch().limit(),
            aperture.gpuvm().base(),
            aperture.gpuvm().limit(),
        ],
        process: [
            u64::from(process.pid()),
            process.start_time_ticks(),
            process.mount_namespace_device(),
            process.mount_namespace_inode(),
        ],
    }
}

fn canonical_physical_device_receipt_id(
    components: &PhysicalDeviceReceiptComponentsV1,
) -> Identity {
    let mut digest = Sha256::new();
    digest.update((M1_PHYSICAL_DEVICE_RECEIPT_DOMAIN_V1.len() as u64).to_le_bytes());
    digest.update(M1_PHYSICAL_DEVICE_RECEIPT_DOMAIN_V1);
    digest.update(components.admission_domain);
    digest.update(components.admission_profile);
    digest.update(components.physical_device_id.to_le_bytes());
    digest.update(components.admission_generation.to_le_bytes());
    digest.update(components.observation_epoch.to_le_bytes());
    digest.update(components.topology_node_id.to_le_bytes());
    digest.update(components.kfd_gpu_id.to_le_bytes());
    digest.update(components.gpu_unique_id.to_le_bytes());
    digest.update(components.pci);
    for value in components.render_descriptor {
        digest.update(value.to_le_bytes());
    }
    for value in components.render_numbers {
        digest.update(value.to_le_bytes());
    }
    for value in components.drm_version {
        digest.update(value.to_le_bytes());
    }
    for value in components.drm_identity {
        digest.update(value.to_le_bytes());
    }
    digest.update(components.aperture_gpu_id.to_le_bytes());
    for value in components.aperture {
        digest.update(value.to_le_bytes());
    }
    for value in components.process {
        digest.update(value.to_le_bytes());
    }
    Identity::new(digest.finalize().into())
}

fn checked_device_receipt(checked: &CheckedGfx942XnackMinusDevice) -> Gfx942DeviceBinding {
    let components = checked_receipt_components(checked);
    Gfx942DeviceBinding::from_physical_receipt(
        canonical_physical_device_receipt_id(&components),
        components.topology_node_id,
        components.kfd_gpu_id,
        components.gpu_unique_id,
        components.admission_generation,
    )
}

/// Non-Clone custody of one checked gfx942 device and its exact service session.
///
/// The private allocation owner was acquired by consuming the same checked fe2
/// token used to derive [`Self::device`]. No raw session extraction is exposed.
///
/// ```compile_fail
/// use ferric_engine::M1CheckedGfx942ServiceDeviceV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1CheckedGfx942ServiceDeviceV1>();
/// ```
///
/// ```compile_fail
/// use ferric_engine::M1CheckedGfx942ServiceDeviceV1;
/// fn escape(owner: M1CheckedGfx942ServiceDeviceV1) {
///     let _ = owner.allocations;
/// }
/// ```
///
/// ```compile_fail
/// use ferric_engine::M1CheckedGfx942ServiceDeviceV1;
/// fn release_twice(owner: M1CheckedGfx942ServiceDeviceV1) {
///     let _ = owner.release_unpublished();
///     let _ = owner.release_unpublished();
/// }
/// ```
#[must_use = "the checked physical device and service session require a consuming transition"]
#[derive(Debug)]
pub struct M1CheckedGfx942ServiceDeviceV1 {
    device: Gfx942DeviceBinding,
    allocations: ServiceAllocationSessionV1,
}

impl M1CheckedGfx942ServiceDeviceV1 {
    /// Returns the detached checked-device receipt, never KFD authority.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }

    /// Returns the number of allocations retained by the exact service session.
    #[must_use]
    pub fn retained_allocation_count(&self) -> usize {
        self.allocations.allocation_count()
    }

    /// Releases every never-published allocation and consumes the service session.
    ///
    /// This is allocation/VM cleanup only. It does not destroy a queue or
    /// prove device, queue, packet, dispatch, completion, or hardware teardown.
    ///
    /// # Errors
    ///
    /// Returns opaque Ferric quarantine retaining the checked receipt and the
    /// exact generic release failure.
    pub fn release_unpublished(
        self,
    ) -> Result<M1UnpublishedAllocationReleaseObservationV1, M1UnpublishedAllocationReleaseFailureV1>
    {
        release_unpublished_allocations(self.device, self.allocations)
    }
}

/// Checked receipt paired with a completed never-published allocation release.
///
/// This observation is not queue or hardware teardown evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1UnpublishedAllocationReleaseObservationV1 {
    device: Gfx942DeviceBinding,
    release: ServiceAllocationReleaseObservationV1,
}

impl M1UnpublishedAllocationReleaseObservationV1 {
    /// Returns the checked physical-device receipt whose service session was consumed.
    #[must_use]
    pub const fn device(self) -> Gfx942DeviceBinding {
        self.device
    }

    /// Returns the redacted generic never-published allocation release observation.
    #[must_use]
    pub const fn release_observation(self) -> ServiceAllocationReleaseObservationV1 {
        self.release
    }
}

/// Opaque quarantine after never-published allocation release failed.
///
/// ```compile_fail
/// use ferric_engine::M1UnpublishedAllocationReleaseFailureV1;
/// fn escape(failure: M1UnpublishedAllocationReleaseFailureV1) {
///     let _ = failure.source;
/// }
/// ```
#[must_use = "failed release retains checked receipt and generic quarantine custody"]
#[derive(Debug)]
pub struct M1UnpublishedAllocationReleaseFailureV1 {
    device: Gfx942DeviceBinding,
    source: Box<ServiceAllocationReleaseFailureV1>,
}

impl M1UnpublishedAllocationReleaseFailureV1 {
    /// Returns the checked physical-device receipt retained in quarantine.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }

    /// Returns the generic release failure by borrow without exposing its retained owner.
    #[must_use = "generic quarantine custody remains retained by the Ferric failure"]
    pub const fn source(&self) -> &ServiceAllocationReleaseFailureV1 {
        &self.source
    }
}

/// Terminal checked-device acquisition failure retaining any generic KFD custody.
///
/// The generic failure can contain a live shared-memory session after owner
/// generation exhaustion. It therefore remains private and cannot be consumed
/// into a raw fe2 session through this API.
#[must_use = "failed checked-device acquisition may retain generic KFD custody"]
pub struct M1CheckedGfx942ServiceDeviceAcquireFailureV1 {
    device: Gfx942DeviceBinding,
    source: Box<ServiceAllocationAcquireErrorV1>,
}

impl fmt::Debug for M1CheckedGfx942ServiceDeviceAcquireFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1CheckedGfx942ServiceDeviceAcquireFailureV1")
            .field("device", &self.device)
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}

impl M1CheckedGfx942ServiceDeviceAcquireFailureV1 {
    /// Returns the checked receipt retained beside the terminal failure.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }

    /// Returns the generic acquisition diagnostic without exposing retained custody.
    #[must_use]
    pub const fn source(&self) -> &ServiceAllocationAcquireErrorV1 {
        &self.source
    }
}

/// Captures a physical receipt and consumes the same checked token into fe2 service custody.
///
/// This establishes device/session equality by construction. It does not
/// allocate model memory, load code, create a queue, submit packets, or claim
/// hardware execution.
///
/// # Errors
///
/// Returns an opaque terminal failure retaining any generic session created by
/// the consuming acquisition attempt.
pub fn acquire_m1_checked_gfx942_service_device_v1(
    checked: CheckedGfx942XnackMinusDevice,
) -> Result<M1CheckedGfx942ServiceDeviceV1, M1CheckedGfx942ServiceDeviceAcquireFailureV1> {
    let device = checked_device_receipt(&checked);
    match ServiceAllocationSessionV1::acquire(checked) {
        Ok(allocations) => Ok(M1CheckedGfx942ServiceDeviceV1 {
            device,
            allocations,
        }),
        Err(source) => Err(M1CheckedGfx942ServiceDeviceAcquireFailureV1 {
            device,
            source: Box::new(source),
        }),
    }
}

/// Closed ownership of initialized model memory on one checked physical device.
///
/// ```compile_fail
/// use ferric_engine::M1DeviceBoundModelMemoryV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1DeviceBoundModelMemoryV1>();
/// ```
///
/// ```compile_fail
/// use ferric_engine::M1DeviceBoundModelMemoryV1;
/// fn escape(owner: M1DeviceBoundModelMemoryV1) {
///     let _ = owner.allocations;
///     let _ = owner.model_memory;
/// }
/// ```
///
/// ```compile_fail
/// use ferric_engine::M1DeviceBoundModelMemoryV1;
/// fn release_twice(owner: M1DeviceBoundModelMemoryV1) {
///     let _ = owner.release_unpublished();
///     let _ = owner.release_unpublished();
/// }
/// ```
#[must_use = "device, service-session, and initialized model-memory custody remain linear"]
#[derive(Debug)]
pub struct M1DeviceBoundModelMemoryV1 {
    device: Gfx942DeviceBinding,
    allocations: ServiceAllocationSessionV1,
    model_memory: BoundModelMemoryAllocationsV1,
}

impl M1DeviceBoundModelMemoryV1 {
    /// Returns the checked physical-device receipt.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }

    /// Returns one inert model allocation identity without exposing its key.
    #[must_use]
    pub const fn allocation_id(
        &self,
        role: ferric_spec::Qwen3ModelRole,
        kind: ferric_build::ModelMemoryAllocationKind,
    ) -> Identity {
        self.model_memory.selected_allocation_identity(role, kind)
    }

    /// Releases all initialized model allocations before any queue exists.
    ///
    /// This consumes model and service-session custody. It is not queue or
    /// hardware teardown and is unavailable after queue ownership is created.
    ///
    /// # Errors
    ///
    /// Returns opaque Ferric quarantine retaining the checked receipt and the
    /// exact generic release failure.
    pub fn release_unpublished(
        self,
    ) -> Result<M1UnpublishedAllocationReleaseObservationV1, M1UnpublishedAllocationReleaseFailureV1>
    {
        release_unpublished_allocations(self.device, self.allocations)
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        Gfx942DeviceBinding,
        ServiceAllocationSessionV1,
        BoundModelMemoryAllocationsV1,
    ) {
        (self.device, self.allocations, self.model_memory)
    }

    pub(crate) fn from_parts(
        device: Gfx942DeviceBinding,
        allocations: ServiceAllocationSessionV1,
        model_memory: BoundModelMemoryAllocationsV1,
    ) -> Self {
        Self {
            device,
            allocations,
            model_memory,
        }
    }
}

/// Stable failure classification for checked-device model initialization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1DeviceModelMemoryAllocationFailureClassV1 {
    /// Host-only plan/image/descriptor rejection occurred before the first service allocation.
    RecoverablePreflight,
    /// Service allocation or final binding began and may retain partial custody.
    TerminalPartialAllocation,
}

/// Model initialization failure retaining the exact checked service owner.
///
/// Partial-allocation failures are intentional quarantine custody: Ferric does
/// not expose the underlying generic session because doing so would reopen the
/// checked device boundary or permit reuse with a partly initialized model.
/// Callers cannot recover or continue it; the only terminal transition is the
/// consuming [`Self::release_unpublished`] cleanup.
///
/// ```compile_fail
/// use ferric_engine::M1DeviceModelMemoryAllocationFailureV1;
/// fn release_twice(failure: M1DeviceModelMemoryAllocationFailureV1) {
///     let _ = failure.release_unpublished();
///     let _ = failure.release_unpublished();
/// }
/// ```
#[must_use = "preflight recovery or opaque partial-allocation custody requires handling"]
#[derive(Debug)]
pub struct M1DeviceModelMemoryAllocationFailureV1 {
    source: Box<InitializedModelMemoryAllocationFailureV1>,
    device: Box<M1CheckedGfx942ServiceDeviceV1>,
    class: M1DeviceModelMemoryAllocationFailureClassV1,
}

impl M1DeviceModelMemoryAllocationFailureV1 {
    /// Classifies retryable host preflight versus terminal partial allocation.
    #[must_use]
    pub const fn class(&self) -> M1DeviceModelMemoryAllocationFailureClassV1 {
        self.class
    }

    /// Returns the exact lower diagnostic without exposing its retained session.
    #[must_use = "the lower model-memory failure remains retained"]
    pub const fn source(&self) -> &InitializedModelMemoryAllocationFailureV1 {
        &self.source
    }

    /// Returns the physical receipt retained beside failure custody.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device.device
    }

    /// Returns the retained allocation count without exposing allocation authority.
    #[must_use]
    pub fn retained_allocation_count(&self) -> usize {
        self.device.retained_allocation_count()
    }

    /// Recovers an unchanged service owner only after host-only preflight rejection.
    ///
    /// # Errors
    ///
    /// Returns the unchanged opaque failure after allocation or binding began.
    #[must_use = "terminal partial-allocation custody remains opaque on rejection"]
    pub fn into_preflight_parts(
        self,
    ) -> Result<
        (
            InitializedModelMemoryAllocationFailureV1,
            M1CheckedGfx942ServiceDeviceV1,
        ),
        Self,
    > {
        if self.class == M1DeviceModelMemoryAllocationFailureClassV1::RecoverablePreflight {
            Ok((*self.source, *self.device))
        } else {
            Err(self)
        }
    }

    /// Releases all allocations retained by a rejected model initialization.
    ///
    /// Both outcomes preserve the exact initialization diagnostic and checked
    /// receipt. This is never-published allocation cleanup only, not queue or
    /// hardware teardown.
    ///
    /// # Errors
    ///
    /// Returns opaque Ferric quarantine retaining the initialization failure,
    /// checked receipt, and exact generic release failure.
    pub fn release_unpublished(
        self,
    ) -> Result<
        M1DeviceModelMemoryFailureReleaseObservationV1,
        M1DeviceModelMemoryFailureReleaseFailureV1,
    > {
        let Self {
            source,
            device,
            class,
        } = self;
        let M1CheckedGfx942ServiceDeviceV1 {
            device,
            allocations,
        } = *device;
        match allocations.release_unpublished() {
            Ok(release) => Ok(M1DeviceModelMemoryFailureReleaseObservationV1 {
                device,
                initialization: source,
                class,
                release,
            }),
            Err(release) => Err(M1DeviceModelMemoryFailureReleaseFailureV1 {
                device,
                initialization: source,
                class,
                release: Box::new(release),
            }),
        }
    }
}

/// Preserved initialization diagnostic after successful unpublished cleanup.
#[must_use = "initialization diagnostic and release observation require handling"]
#[derive(Debug)]
pub struct M1DeviceModelMemoryFailureReleaseObservationV1 {
    device: Gfx942DeviceBinding,
    initialization: Box<InitializedModelMemoryAllocationFailureV1>,
    class: M1DeviceModelMemoryAllocationFailureClassV1,
    release: ServiceAllocationReleaseObservationV1,
}

impl M1DeviceModelMemoryFailureReleaseObservationV1 {
    /// Returns the checked physical-device receipt whose session was consumed.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }

    /// Returns the original initialization failure by borrow.
    #[must_use = "the original initialization failure remains retained"]
    pub const fn initialization(&self) -> &InitializedModelMemoryAllocationFailureV1 {
        &self.initialization
    }

    /// Returns the original preflight/partial-allocation classification.
    #[must_use]
    pub const fn class(&self) -> M1DeviceModelMemoryAllocationFailureClassV1 {
        self.class
    }

    /// Returns the redacted generic never-published release observation.
    #[must_use]
    pub const fn release_observation(&self) -> ServiceAllocationReleaseObservationV1 {
        self.release
    }
}

/// Opaque quarantine preserving a failed initialization and failed cleanup.
///
/// ```compile_fail
/// use ferric_engine::M1DeviceModelMemoryFailureReleaseFailureV1;
/// fn escape(failure: M1DeviceModelMemoryFailureReleaseFailureV1) {
///     let _ = failure.release;
/// }
/// ```
#[must_use = "initialization and release quarantine custody require handling"]
#[derive(Debug)]
pub struct M1DeviceModelMemoryFailureReleaseFailureV1 {
    device: Gfx942DeviceBinding,
    initialization: Box<InitializedModelMemoryAllocationFailureV1>,
    class: M1DeviceModelMemoryAllocationFailureClassV1,
    release: Box<ServiceAllocationReleaseFailureV1>,
}

impl M1DeviceModelMemoryFailureReleaseFailureV1 {
    /// Returns the checked physical-device receipt retained in quarantine.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }

    /// Returns the original initialization failure by borrow.
    #[must_use = "the original initialization failure remains retained"]
    pub const fn initialization(&self) -> &InitializedModelMemoryAllocationFailureV1 {
        &self.initialization
    }

    /// Returns the original preflight/partial-allocation classification.
    #[must_use]
    pub const fn class(&self) -> M1DeviceModelMemoryAllocationFailureClassV1 {
        self.class
    }

    /// Returns the generic release failure without exposing quarantined authority.
    #[must_use = "generic quarantine custody remains retained by the Ferric failure"]
    pub const fn release_failure(&self) -> &ServiceAllocationReleaseFailureV1 {
        &self.release
    }
}

fn release_unpublished_allocations(
    device: Gfx942DeviceBinding,
    allocations: ServiceAllocationSessionV1,
) -> Result<M1UnpublishedAllocationReleaseObservationV1, M1UnpublishedAllocationReleaseFailureV1> {
    match allocations.release_unpublished() {
        Ok(release) => Ok(M1UnpublishedAllocationReleaseObservationV1 { device, release }),
        Err(source) => Err(M1UnpublishedAllocationReleaseFailureV1 {
            device,
            source: Box::new(source),
        }),
    }
}

/// Allocates initialized model memory on the exact checked service device.
///
/// Success closes the physical receipt, service session, and model keys into a
/// single owner accepted by the production KV partition boundary. The generic
/// initializer remains responsible for complete-image allocation and mapping.
///
/// # Errors
///
/// Host-only plan, image, and descriptor rejection returns recoverable
/// checked-device custody because the lower initializer constructs every
/// descriptor before its first service allocation. Once a service allocation
/// is attempted, the failure retains opaque terminal custody because earlier
/// allocations may remain live.
pub fn allocate_initialized_m1_model_memory_on_device_v1(
    mut device: M1CheckedGfx942ServiceDeviceV1,
    plan: AddresslessModelMemoryPlan,
    target_prepacked_weights: Box<[u8]>,
    draft_prepacked_weights: Box<[u8]>,
) -> Result<M1DeviceBoundModelMemoryV1, M1DeviceModelMemoryAllocationFailureV1> {
    match allocate_initialized_model_memory_v1(
        &mut device.allocations,
        plan,
        target_prepacked_weights,
        draft_prepacked_weights,
    ) {
        Ok(model_memory) => Ok(M1DeviceBoundModelMemoryV1 {
            device: device.device,
            allocations: device.allocations,
            model_memory,
        }),
        Err(source) => {
            let class = match source.error() {
                InitializedModelMemoryAllocationErrorV1::Preflight(_)
                | InitializedModelMemoryAllocationErrorV1::Descriptor { .. } => {
                    M1DeviceModelMemoryAllocationFailureClassV1::RecoverablePreflight
                }
                InitializedModelMemoryAllocationErrorV1::Allocation { .. }
                | InitializedModelMemoryAllocationErrorV1::Binding(_) => {
                    M1DeviceModelMemoryAllocationFailureClassV1::TerminalPartialAllocation
                }
            };
            Err(M1DeviceModelMemoryAllocationFailureV1 {
                source: Box::new(source),
                device: Box::new(device),
                class,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn components() -> PhysicalDeviceReceiptComponentsV1 {
        PhysicalDeviceReceiptComponentsV1 {
            admission_domain: [1; 32],
            admission_profile: [2; 32],
            physical_device_id: 3,
            admission_generation: 4,
            observation_epoch: 5,
            topology_node_id: 6,
            kfd_gpu_id: 7,
            gpu_unique_id: 8,
            pci: [9, 10, 11, 12, 13],
            render_descriptor: [14, 15, 16],
            render_numbers: [17, 18, 19],
            drm_version: [-20, 21, -22],
            drm_identity: [23, 24, 25, 26, 27, 28, 29],
            aperture_gpu_id: 30,
            aperture: [31, 32, 33, 34, 35, 36],
            process: [37, 38, 39, 40],
        }
    }

    fn assert_drift(mut mutate: impl FnMut(&mut PhysicalDeviceReceiptComponentsV1)) {
        let expected = canonical_physical_device_receipt_id(&components());
        let mut drifted = components();
        mutate(&mut drifted);
        assert_ne!(canonical_physical_device_receipt_id(&drifted), expected);
    }

    #[test]
    fn every_checked_receipt_component_is_identity_bound() {
        assert_drift(|value| value.admission_domain[0] ^= 1);
        assert_drift(|value| value.admission_profile[0] ^= 1);
        assert_drift(|value| value.physical_device_id += 1);
        assert_drift(|value| value.admission_generation += 1);
        assert_drift(|value| value.observation_epoch += 1);
        assert_drift(|value| value.topology_node_id += 1);
        assert_drift(|value| value.kfd_gpu_id += 1);
        assert_drift(|value| value.gpu_unique_id += 1);
        for index in 0..5 {
            assert_drift(|value| value.pci[index] ^= 1);
        }
        for index in 0..3 {
            assert_drift(|value| value.render_descriptor[index] += 1);
            assert_drift(|value| value.render_numbers[index] += 1);
        }
        for index in 0..3 {
            assert_drift(|value| value.drm_version[index] += 1);
        }
        for index in 0..7 {
            assert_drift(|value| value.drm_identity[index] += 1);
        }
        assert_drift(|value| value.aperture_gpu_id += 1);
        for index in 0..6 {
            assert_drift(|value| value.aperture[index] += 1);
        }
        for index in 0..4 {
            assert_drift(|value| value.process[index] += 1);
        }
    }

    #[test]
    fn canonical_checked_receipt_is_deterministic_and_present() {
        let first = canonical_physical_device_receipt_id(&components());
        let second = canonical_physical_device_receipt_id(&components());
        assert_eq!(first, second);
        assert!(first.is_present());
    }

    #[test]
    #[ignore = "requires the admitted Linux MI300X KFD profile and explicit GPU unique ID"]
    fn real_kfd_acquisition_preserves_checked_device_observations() {
        use fe2o3_kfd::{DeviceSelector, OpenedKfd};

        let unique_id = std::env::var("FERRIC_M1_GPU_UNIQUE_ID")
            .expect("set FERRIC_M1_GPU_UNIQUE_ID for the selected MI300X")
            .parse::<u64>()
            .expect("FERRIC_M1_GPU_UNIQUE_ID must be a u64");
        let checked = OpenedKfd::open_default()
            .unwrap()
            .admit_uapi()
            .unwrap()
            .bind_gfx942_xnack_minus(DeviceSelector::UniqueId(unique_id))
            .unwrap();
        let expected_node = checked.observation().topology_node_id();
        let expected_gpu = checked.observation().kfd_gpu_id();
        let expected_unique = checked.observation().unique_id();
        let owner = acquire_m1_checked_gfx942_service_device_v1(checked).unwrap();
        assert_eq!(owner.device().node_id(), expected_node);
        assert_eq!(owner.device().kfd_gpu_id(), expected_gpu);
        assert_eq!(owner.device().gpu_unique_id(), expected_unique);
        assert_eq!(owner.retained_allocation_count(), 0);
        let released = owner.release_unpublished().unwrap();
        assert_eq!(released.device().node_id(), expected_node);
        assert_eq!(released.release_observation().allocation_count(), 0);
    }
}
