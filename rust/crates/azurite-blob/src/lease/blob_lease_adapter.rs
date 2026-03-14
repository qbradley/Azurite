/// Mirrors TypeScript `src/blob/lease/BlobLeaseAdapter.ts`.
///
/// Fidelity: silently defaults missing `blob.properties.leaseState` to `Available`
/// and missing `blob.properties.leaseStatus` to `Unlocked` (the TS commented-out throw
/// paths are preserved as comments).
use crate::generated::artifacts::models::GeneratedValue;
use crate::lease::i_lease_state::{ILease, LeaseStateType, LeaseStatusType};
use crate::persistence::BlobModel;

pub struct BlobLeaseAdapter;

impl BlobLeaseAdapter {
    /// Back-compat constructor for legacy call sites that still use `new(&blob)`.
    pub fn new(blob: &BlobModel) -> ILease {
        let mut cloned = blob.clone();
        Self::from_blob(&mut cloned)
    }

    /// Constructs an `ILease` view of the blob's current lease state.
    pub fn from_blob(blob: &mut BlobModel) -> ILease {
        // Default missing leaseState / leaseStatus rather than throwing.
        if blob
            .properties
            .get("leaseState")
            .and_then(|v| v.as_string())
            .is_none()
        {
            blob.properties.insert(
                "leaseState".to_string(),
                GeneratedValue::String(LeaseStateType::Available.to_string()),
            );
            // throw RangeError(`BlobLeaseAdapter:constructor() blob leaseState cannot be undefined.`);
        }

        if blob
            .properties
            .get("leaseStatus")
            .and_then(|v| v.as_string())
            .is_none()
        {
            blob.properties.insert(
                "leaseStatus".to_string(),
                GeneratedValue::String(LeaseStatusType::Unlocked.to_string()),
            );
            // throw RangeError(`BlobLeaseAdapter:constructor() blob leaseStatus cannot be undefined.`);
        }

        ILease {
            leaseId: blob.leaseId.clone(),
            leaseState: blob
                .properties
                .get("leaseState")
                .and_then(|v| v.as_string()),
            leaseStatus: blob
                .properties
                .get("leaseStatus")
                .and_then(|v| v.as_string()),
            leaseDurationType: blob
                .properties
                .get("leaseDuration")
                .and_then(|v| v.as_string()),
            leaseDurationSeconds: blob.leaseDurationSeconds,
            leaseExpireTime: blob.leaseExpireTime,
            leaseBreakTime: blob.leaseBreakTime,
        }
    }
}
