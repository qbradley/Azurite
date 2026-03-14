/// Mirrors TypeScript `src/blob/lease/ContainerLeaseAdapter.ts`.
///
/// Fidelity: throws `RangeError` (mapped to a `String` error) when container
/// `leaseState` or `leaseStatus` are missing — unlike `BlobLeaseAdapter` which
/// silently defaults.
use crate::lease::i_lease_state::{ILease, LeaseStateType, LeaseStatusType};
use crate::persistence::ContainerModel;

pub struct ContainerLeaseAdapter;

impl ContainerLeaseAdapter {
    /// Back-compat constructor for legacy call sites that still use `new(&container)`.
    pub fn new(container: &ContainerModel) -> ILease {
        Self::from_container(container).unwrap_or_else(|_| ILease {
            leaseId: container.leaseId.clone(),
            leaseState: Some(LeaseStateType::Available.to_string()),
            leaseStatus: Some(LeaseStatusType::Unlocked.to_string()),
            leaseDurationType: container
                .properties
                .get("leaseDuration")
                .and_then(|v| v.as_string()),
            leaseDurationSeconds: container.leaseDurationSeconds,
            leaseExpireTime: container.leaseExpireTime,
            leaseBreakTime: container.leaseBreakTime,
        })
    }

    /// Constructs an `ILease` view of the container's current lease state.
    ///
    /// Returns `Err(String)` if `leaseState` or `leaseStatus` are absent from
    /// `container.properties` (mirrors the TS `RangeError` throws).
    pub fn from_container(container: &ContainerModel) -> Result<ILease, String> {
        let lease_state = container
            .properties
            .get("leaseState")
            .and_then(|v| v.as_string());
        if lease_state.is_none() {
            return Err(
                "ContainerLeaseAdapter:constructor() container leaseState cannot be undefined."
                    .to_string(),
            );
        }

        let lease_status = container
            .properties
            .get("leaseStatus")
            .and_then(|v| v.as_string());
        if lease_status.is_none() {
            return Err(
                "ContainerLeaseAdapter:constructor() container leaseStatus cannot be undefined."
                    .to_string(),
            );
        }

        Ok(ILease {
            leaseId: container.leaseId.clone(),
            leaseState: lease_state,
            leaseStatus: lease_status,
            leaseDurationType: container
                .properties
                .get("leaseDuration")
                .and_then(|v| v.as_string()),
            leaseDurationSeconds: container.leaseDurationSeconds,
            leaseExpireTime: container.leaseExpireTime,
            leaseBreakTime: container.leaseBreakTime,
        })
    }
}
