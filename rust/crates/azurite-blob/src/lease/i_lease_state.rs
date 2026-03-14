/// Mirrors TypeScript `src/blob/lease/ILeaseState.ts`.
use chrono::{DateTime, Utc};

use crate::errors::StorageError;
use crate::generated::context::Context;

/// Lease state string constants — mirrors TypeScript `LeaseStateType` enum members.
#[allow(non_snake_case, non_upper_case_globals)]
pub mod LeaseStateType {
    pub const Available: &str = "available";
    pub const Leased: &str = "leased";
    pub const Expired: &str = "expired";
    pub const Breaking: &str = "breaking";
    pub const Broken: &str = "broken";
}

/// Lease status string constants — mirrors TypeScript `LeaseStatusType` enum members.
#[allow(non_snake_case, non_upper_case_globals)]
pub mod LeaseStatusType {
    pub const Locked: &str = "locked";
    pub const Unlocked: &str = "unlocked";
}

/// Lease duration string constants — mirrors TypeScript `LeaseDurationType` enum members.
#[allow(non_snake_case, non_upper_case_globals)]
pub mod LeaseDurationType {
    pub const Infinite: &str = "infinite";
    pub const Fixed: &str = "fixed";
}

/// Mirrors TypeScript `ILease` interface.
#[allow(non_snake_case)]
#[derive(Debug, Clone, Default)]
pub struct ILease {
    pub leaseId: Option<String>,
    /// Corresponds to `Models.LeaseStateType` (String alias).
    pub leaseState: Option<String>,
    /// Corresponds to `Models.LeaseStatusType` (String alias).
    pub leaseStatus: Option<String>,
    /// Corresponds to `Models.LeaseDurationType` (String alias).
    pub leaseDurationType: Option<String>,
    /// Positive integer (fixed lease seconds) or -1 (infinite).
    pub leaseDurationSeconds: Option<i64>,
    pub leaseExpireTime: Option<DateTime<Utc>>,
    pub leaseBreakTime: Option<DateTime<Utc>>,
}

/// Mirrors TypeScript `ILeaseValidator` interface.
pub trait ILeaseValidator {
    fn validate(&self, lease: &ILease, context: &Context) -> Result<(), StorageError>;
}

/// Mirrors TypeScript `ILeaseSyncer<T>` interface.
pub trait ILeaseSyncer<T> {
    fn sync(&mut self, lease: &ILease) -> T;
}

/// Mirrors TypeScript `ILeaseState` interface.
///
/// The TypeScript interface has a generic `sync<T>(syncer: ILeaseSyncer<T>): T` method
/// which is not object-safe in Rust.  We expose `lease()` instead so callers can do
/// `syncer.sync(state.lease())` directly, preserving the exact same semantics without
/// requiring a generic method on the trait object.
pub trait ILeaseState: Send + Sync {
    fn lease(&self) -> &ILease;

    fn acquire(
        &self,
        duration: i64,
        proposed_lease_id: Option<&str>,
    ) -> Result<Box<dyn ILeaseState>, StorageError>;

    fn break_lease(&self, break_period: Option<i64>) -> Result<Box<dyn ILeaseState>, StorageError>;

    fn renew(&self, lease_id: &str) -> Result<Box<dyn ILeaseState>, StorageError>;

    fn change(
        &self,
        lease_id: &str,
        proposed_lease_id: &str,
    ) -> Result<Box<dyn ILeaseState>, StorageError>;

    fn release(&self, lease_id: &str) -> Result<Box<dyn ILeaseState>, StorageError>;

    /// Mirrors `LeaseStateBase.validate()`.  Runs the validator and returns `Ok(())` on
    /// success; the caller continues to use the same state object (fidelity: TS returns `this`).
    fn validate(&self, validator: &dyn ILeaseValidator) -> Result<(), StorageError>;
}
