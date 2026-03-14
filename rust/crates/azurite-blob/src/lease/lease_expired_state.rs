/// Mirrors TypeScript `src/blob/lease/LeaseExpiredState.ts`.
///
/// Fidelity notes:
/// - Constructor accepts either `Expired` or an already-expired `Leased` (Fixed) lease
///   and normalises to `Expired` state.
/// - `renew()` ignores its caller-supplied `lease_id` argument and uses the stored
///   `self.base.lease.leaseId` and `leaseDurationSeconds`.  This matches the TS
///   implementation which declares `renew()` with zero parameters (signature mismatch
///   with the interface).
/// - `change()` always throws LeaseNotPresent (no parameters in TS impl).
use uuid::Uuid;

use crate::errors::storage_error_factory::StorageErrorFactory;
use crate::generated::context::Context;
use crate::lease::i_lease_state::{ILease, ILeaseState, ILeaseValidator, LeaseDurationType, LeaseStateType, LeaseStatusType};
use crate::lease::lease_state_base::LeaseStateBase;

pub struct LeaseExpiredState {
    pub base: LeaseStateBase,
}

impl LeaseExpiredState {
    pub fn new(lease: ILease, context: Context) -> Result<Self, String> {
        if context.startTime().is_none() {
            return Err(
                "LeaseExpiredState:constructor() error, context.startTime is undefined.".into(),
            );
        }

        match lease.leaseState.as_deref() {
            Some(s) if s == LeaseStateType::Expired => {
                if lease.leaseStatus.as_deref() != Some(LeaseStatusType::Unlocked) {
                    return Err(format!(
                        "LeaseExpiredState:constructor() error, incoming lease status {:?} is not {}.",
                        lease.leaseStatus,
                        LeaseStatusType::Unlocked
                    ));
                }
                if lease.leaseId.is_none() {
                    return Err(format!(
                        "LeaseExpiredState:constructor() error, incoming leaseId {:?} should not be undefined.",
                        lease.leaseId
                    ));
                }
                if lease.leaseExpireTime.is_some() {
                    return Err(format!(
                        "LeaseExpiredState:constructor() error, incoming leaseExpireTime {:?} is undefined.",
                        lease.leaseExpireTime
                    ));
                }
                if lease.leaseDurationSeconds.is_none() || lease.leaseDurationSeconds == Some(-1) {
                    return Err(format!(
                        "LeaseExpiredState:constructor() error, incoming leaseDurationSeconds {:?} is undefined or -1 (infinite).",
                        lease.leaseDurationSeconds
                    ));
                }
                if lease.leaseDurationType.is_some() {
                    return Err(format!(
                        "LeaseExpiredState:constructor() error, incoming leaseDurationType {:?} is not undefined.",
                        lease.leaseDurationType
                    ));
                }
                if lease.leaseBreakTime.is_some() {
                    return Err(format!(
                        "LeaseExpiredState:constructor() error, incoming leaseBreakTime {:?} is not undefined.",
                        lease.leaseBreakTime
                    ));
                }
                // Deep copy
                Ok(Self {
                    base: LeaseStateBase::new(lease, context),
                })
            }
            Some(s) if s == LeaseStateType::Leased => {
                // An expired fixed lease — normalise to Expired
                if lease.leaseStatus.as_deref() != Some(LeaseStatusType::Locked) {
                    return Err(format!(
                        "LeaseExpiredState:constructor() error, incoming lease status {:?} is not {}.",
                        lease.leaseStatus,
                        LeaseStatusType::Locked
                    ));
                }
                if lease.leaseId.is_none() {
                    return Err(format!(
                        "LeaseExpiredState:constructor() error, incoming leaseId {:?} should not be undefined.",
                        lease.leaseId
                    ));
                }
                let start = context.startTime().unwrap();
                if lease.leaseExpireTime.is_none() || start < lease.leaseExpireTime.unwrap() {
                    return Err(format!(
                        "LeaseExpiredState:constructor() error, incoming leaseExpireTime {:?} is undefined, or larger than current time {:?}.",
                        lease.leaseExpireTime, start
                    ));
                }
                if lease.leaseDurationSeconds.is_none() || lease.leaseDurationSeconds == Some(-1) {
                    return Err(format!(
                        "LeaseExpiredState:constructor() error, incoming leaseDurationSeconds {:?} is undefined or -1 (infinite).",
                        lease.leaseDurationSeconds
                    ));
                }
                if lease.leaseDurationType.as_deref() != Some(LeaseDurationType::Fixed) {
                    return Err(format!(
                        "LeaseExpiredState:constructor() error, incoming leaseDurationType {:?} is not {}.",
                        lease.leaseDurationType,
                        LeaseDurationType::Fixed
                    ));
                }
                if lease.leaseBreakTime.is_some() {
                    return Err(format!(
                        "LeaseExpiredState:constructor() error, incoming leaseBreakTime {:?} is not undefined.",
                        lease.leaseBreakTime
                    ));
                }
                // Normalise Leased → Expired
                Ok(Self {
                    base: LeaseStateBase::new(
                        ILease {
                            leaseId: lease.leaseId,
                            leaseState: Some(LeaseStateType::Expired.to_string()),
                            leaseStatus: Some(LeaseStatusType::Unlocked.to_string()),
                            leaseDurationType: None,
                            leaseDurationSeconds: lease.leaseDurationSeconds,
                            leaseExpireTime: None,
                            leaseBreakTime: None,
                        },
                        context,
                    ),
                })
            }
            other => Err(format!(
                "LeaseExpiredState:constructor() error, incoming lease state {:?} is neither {} or {}.",
                other,
                LeaseStateType::Expired,
                LeaseStateType::Leased
            )),
        }
    }
}

impl ILeaseState for LeaseExpiredState {
    fn lease(&self) -> &ILease {
        &self.base.lease
    }

    fn acquire(
        &self,
        duration: i64,
        proposed_lease_id: Option<&str>,
    ) -> Result<Box<dyn ILeaseState>, crate::errors::StorageError> {
        if (duration < 15 || duration > 60) && duration != -1 {
            return Err(StorageErrorFactory::getInvalidLeaseDuration(
                self.base.context.contextId().as_deref(),
            ));
        }

        let lease_id = proposed_lease_id
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let start = self.base.context.startTime().unwrap();
        let ctx = self.base.context.clone();

        if duration == -1 {
            Ok(Box::new(
                crate::lease::lease_leased_state::LeaseLeasedState::new(
                    ILease {
                        leaseId: Some(lease_id),
                        leaseState: Some(LeaseStateType::Leased.to_string()),
                        leaseStatus: Some(LeaseStatusType::Locked.to_string()),
                        leaseDurationType: Some(LeaseDurationType::Infinite.to_string()),
                        leaseDurationSeconds: None,
                        leaseExpireTime: None,
                        leaseBreakTime: None,
                    },
                    ctx,
                )
                .map_err(|e| crate::errors::StorageError::new(500, "InternalError", &e, "", std::collections::BTreeMap::new()))?,
            ))
        } else {
            let expire = start + chrono::Duration::seconds(duration);
            Ok(Box::new(
                crate::lease::lease_leased_state::LeaseLeasedState::new(
                    ILease {
                        leaseId: Some(lease_id),
                        leaseState: Some(LeaseStateType::Leased.to_string()),
                        leaseStatus: Some(LeaseStatusType::Locked.to_string()),
                        leaseDurationType: Some(LeaseDurationType::Fixed.to_string()),
                        leaseDurationSeconds: Some(duration),
                        leaseExpireTime: Some(expire),
                        leaseBreakTime: None,
                    },
                    ctx,
                )
                .map_err(|e| crate::errors::StorageError::new(500, "InternalError", &e, "", std::collections::BTreeMap::new()))?,
            ))
        }
    }

    fn break_lease(
        &self,
        _break_period: Option<i64>,
    ) -> Result<Box<dyn ILeaseState>, crate::errors::StorageError> {
        Ok(Box::new(
            crate::lease::lease_broken_state::LeaseBrokenState::new(
                ILease {
                    leaseId: self.base.lease.leaseId.clone(),
                    leaseState: Some(LeaseStateType::Broken.to_string()),
                    leaseStatus: Some(LeaseStatusType::Unlocked.to_string()),
                    leaseDurationType: None,
                    leaseDurationSeconds: None,
                    leaseExpireTime: None,
                    leaseBreakTime: None,
                },
                self.base.context.clone(),
            )
            .map_err(|e| crate::errors::StorageError::new(500, "InternalError", &e, "", std::collections::BTreeMap::new()))?,
        ))
    }

    /// Fidelity: ignores `_lease_id`; uses stored `self.base.lease.leaseId` and
    /// `leaseDurationSeconds` directly (TS implementation declares zero parameters).
    fn renew(&self, _lease_id: &str) -> Result<Box<dyn ILeaseState>, crate::errors::StorageError> {
        let start = self.base.context.startTime().unwrap();
        let duration = self.base.lease.leaseDurationSeconds.unwrap();
        let expire = start + chrono::Duration::seconds(duration);

        Ok(Box::new(
            crate::lease::lease_leased_state::LeaseLeasedState::new(
                ILease {
                    leaseId: self.base.lease.leaseId.clone(),
                    leaseState: Some(LeaseStateType::Leased.to_string()),
                    leaseStatus: Some(LeaseStatusType::Locked.to_string()),
                    leaseDurationType: Some(LeaseDurationType::Fixed.to_string()),
                    leaseDurationSeconds: Some(duration),
                    leaseExpireTime: Some(expire),
                    leaseBreakTime: None,
                },
                self.base.context.clone(),
            )
            .map_err(|e| crate::errors::StorageError::new(500, "InternalError", &e, "", std::collections::BTreeMap::new()))?,
        ))
    }

    /// Fidelity: TS impl has zero parameters; always throws LeaseNotPresent.
    fn change(
        &self,
        _lease_id: &str,
        _proposed_lease_id: &str,
    ) -> Result<Box<dyn ILeaseState>, crate::errors::StorageError> {
        Err(StorageErrorFactory::getLeaseNotPresentWithLeaseOperation(
            self.base.context.contextId().as_deref(),
        ))
    }

    fn release(
        &self,
        lease_id: &str,
    ) -> Result<Box<dyn ILeaseState>, crate::errors::StorageError> {
        if self.base.lease.leaseId.as_deref() != Some(lease_id) {
            return Err(StorageErrorFactory::getLeaseIdMismatchWithLeaseOperation(
                self.base.context.contextId().as_deref(),
            ));
        }

        Ok(Box::new(
            crate::lease::lease_available_state::LeaseAvailableState::new(
                ILease {
                    leaseId: None,
                    leaseState: Some(LeaseStateType::Available.to_string()),
                    leaseStatus: Some(LeaseStatusType::Unlocked.to_string()),
                    leaseDurationType: None,
                    leaseDurationSeconds: None,
                    leaseExpireTime: None,
                    leaseBreakTime: None,
                },
                self.base.context.clone(),
            )
            .map_err(|e| crate::errors::StorageError::new(500, "InternalError", &e, "", std::collections::BTreeMap::new()))?,
        ))
    }

    fn validate(&self, validator: &dyn ILeaseValidator) -> Result<(), crate::errors::StorageError> {
        self.base.validate(validator)
    }
}
