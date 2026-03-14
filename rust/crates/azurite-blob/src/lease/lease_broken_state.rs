/// Mirrors TypeScript `src/blob/lease/LeaseBrokenState.ts`.
///
/// Fidelity notes:
/// - `renew(proposed_lease_id)` compares argument to stored leaseId:
///   match ⇒ LeaseIsBrokenAndCannotBeRenewed; otherwise ⇒ LeaseIdMismatch.
/// - `change()` takes no effective parameters (TS impl has zero params) so
///   both arguments are ignored; always throws LeaseNotPresent.
use uuid::Uuid;

use crate::errors::storage_error_factory::StorageErrorFactory;
use crate::generated::context::Context;
use crate::lease::i_lease_state::{
    ILease, ILeaseState, ILeaseValidator, LeaseDurationType, LeaseStateType, LeaseStatusType,
};
use crate::lease::lease_state_base::LeaseStateBase;

pub struct LeaseBrokenState {
    pub base: LeaseStateBase,
}

impl LeaseBrokenState {
    pub fn new(lease: ILease, context: Context) -> Result<Self, String> {
        if context.startTime().is_none() {
            return Err(
                "LeaseBrokenState:constructor() error, context.startTime is undefined.".into(),
            );
        }

        match lease.leaseState.as_deref() {
            Some(s) if s == LeaseStateType::Broken => {
                if lease.leaseStatus.as_deref() != Some(LeaseStatusType::Unlocked) {
                    return Err(format!(
                        "LeaseBrokenState:constructor() error, incoming lease status {:?} is not {}.",
                        lease.leaseStatus,
                        LeaseStatusType::Unlocked
                    ));
                }
                if lease.leaseId.is_none() {
                    return Err(format!(
                        "LeaseBrokenState:constructor() error, incoming leaseId {:?} should not be undefined.",
                        lease.leaseId
                    ));
                }
                if lease.leaseExpireTime.is_some() {
                    return Err(format!(
                        "LeaseBrokenState:constructor() error, incoming leaseExpireTime {:?} is not undefined.",
                        lease.leaseExpireTime
                    ));
                }
                if lease.leaseDurationSeconds.is_some() {
                    return Err(format!(
                        "LeaseBrokenState:constructor() error, incoming leaseDurationSeconds {:?} is not undefined.",
                        lease.leaseDurationSeconds
                    ));
                }
                if lease.leaseDurationType.is_some() {
                    return Err(format!(
                        "LeaseBrokenState:constructor() error, incoming leaseDurationType {:?} is not undefined.",
                        lease.leaseDurationType
                    ));
                }
                if lease.leaseBreakTime.is_some() {
                    return Err(format!(
                        "LeaseBrokenState:constructor() error, incoming leaseBreakTime {:?} is not undefined.",
                        lease.leaseBreakTime
                    ));
                }
                Ok(Self {
                    base: LeaseStateBase::new(lease, context),
                })
            }
            Some(s) if s == LeaseStateType::Breaking => {
                // An expired-break transitions to Broken — normalise to Broken state.
                if lease.leaseStatus.as_deref() != Some(LeaseStatusType::Locked) {
                    return Err(format!(
                        "LeaseBrokenState:constructor() error, incoming lease status {:?} is not {}.",
                        lease.leaseStatus,
                        LeaseStatusType::Locked
                    ));
                }
                if lease.leaseId.is_none() {
                    return Err(format!(
                        "LeaseBrokenState:constructor() error, incoming leaseId {:?} should not be undefined.",
                        lease.leaseId
                    ));
                }
                if lease.leaseExpireTime.is_some() {
                    return Err(format!(
                        "LeaseBrokenState:constructor() error, incoming leaseExpireTime {:?} is not undefined.",
                        lease.leaseExpireTime
                    ));
                }
                if lease.leaseDurationSeconds.is_some() {
                    return Err(format!(
                        "LeaseBrokenState:constructor() error, incoming leaseDurationSeconds {:?} is not undefined.",
                        lease.leaseDurationSeconds
                    ));
                }
                if lease.leaseDurationType.is_some() {
                    return Err(format!(
                        "LeaseBrokenState:constructor() error, incoming leaseDurationType {:?} is not undefined.",
                        lease.leaseDurationType
                    ));
                }
                let break_time = lease.leaseBreakTime;
                let start = context.startTime().unwrap();
                if break_time.is_none() || start < break_time.unwrap() {
                    return Err(format!(
                        "LeaseBrokenState:constructor() error, incoming leaseBreakTime {:?} is undefined, or larger than current time {:?}.",
                        break_time, start
                    ));
                }
                // Normalise Breaking → Broken
                Ok(Self {
                    base: LeaseStateBase::new(
                        ILease {
                            leaseId: lease.leaseId,
                            leaseState: Some(LeaseStateType::Broken.to_string()),
                            leaseStatus: Some(LeaseStatusType::Unlocked.to_string()),
                            leaseDurationType: None,
                            leaseDurationSeconds: None,
                            leaseExpireTime: None,
                            leaseBreakTime: None,
                        },
                        context,
                    ),
                })
            }
            other => Err(format!(
                "LeaseBrokenState:constructor() error, incoming lease state {:?} is not {} or {}.",
                other,
                LeaseStateType::Broken,
                LeaseStateType::Breaking
            )),
        }
    }
}

impl ILeaseState for LeaseBrokenState {
    fn lease(&self) -> &ILease {
        &self.base.lease
    }

    fn acquire(
        &self,
        duration: i64,
        proposed_lease_id: Option<&str>,
    ) -> Result<Box<dyn ILeaseState>, crate::errors::StorageError> {
        if !(15..=60).contains(&duration) && duration != -1 {
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
                .map_err(|e| {
                    crate::errors::StorageError::new(
                        500,
                        "InternalError",
                        &e,
                        "",
                        std::collections::BTreeMap::new(),
                    )
                })?,
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
                .map_err(|e| {
                    crate::errors::StorageError::new(
                        500,
                        "InternalError",
                        &e,
                        "",
                        std::collections::BTreeMap::new(),
                    )
                })?,
            ))
        }
    }

    /// Mirrors TS `LeaseBrokenState.break()`: always returns `this` unchanged.
    fn break_lease(
        &self,
        _break_period: Option<i64>,
    ) -> Result<Box<dyn ILeaseState>, crate::errors::StorageError> {
        Ok(Box::new(
            LeaseBrokenState::new(self.base.lease.clone(), self.base.context.clone()).map_err(
                |e| {
                    crate::errors::StorageError::new(
                        500,
                        "InternalError",
                        &e,
                        "",
                        std::collections::BTreeMap::new(),
                    )
                },
            )?,
        ))
    }

    /// Fidelity: compares argument to stored leaseId.  Match ⇒ IsBrokenAndCannotBeRenewed;
    /// mismatch ⇒ IdMismatch.
    fn renew(
        &self,
        proposed_lease_id: &str,
    ) -> Result<Box<dyn ILeaseState>, crate::errors::StorageError> {
        if proposed_lease_id == self.base.lease.leaseId.as_deref().unwrap_or("") {
            Err(StorageErrorFactory::getLeaseIsBrokenAndCannotBeRenewed(
                self.base.context.contextId().as_deref(),
            ))
        } else {
            Err(StorageErrorFactory::getLeaseIdMismatchWithLeaseOperation(
                self.base.context.contextId().as_deref(),
            ))
        }
    }

    /// Fidelity: TS impl has zero effective parameters; always throws LeaseNotPresent.
    fn change(
        &self,
        _lease_id: &str,
        _proposed_lease_id: &str,
    ) -> Result<Box<dyn ILeaseState>, crate::errors::StorageError> {
        Err(StorageErrorFactory::getLeaseNotPresentWithLeaseOperation(
            self.base.context.contextId().as_deref(),
        ))
    }

    fn release(&self, lease_id: &str) -> Result<Box<dyn ILeaseState>, crate::errors::StorageError> {
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
            .map_err(|e| {
                crate::errors::StorageError::new(
                    500,
                    "InternalError",
                    &e,
                    "",
                    std::collections::BTreeMap::new(),
                )
            })?,
        ))
    }

    fn validate(&self, validator: &dyn ILeaseValidator) -> Result<(), crate::errors::StorageError> {
        self.base.validate(validator)
    }
}
