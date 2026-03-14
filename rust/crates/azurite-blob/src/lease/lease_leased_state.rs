/// Mirrors TypeScript `src/blob/lease/LeaseLeasedState.ts`.
use uuid::Uuid;

use azurite_common::utils::utils::minDate;

use crate::errors::storage_error_factory::StorageErrorFactory;
use crate::generated::context::Context;
use crate::lease::i_lease_state::{
    ILease, ILeaseState, ILeaseValidator, LeaseDurationType, LeaseStateType, LeaseStatusType,
};
use crate::lease::lease_state_base::LeaseStateBase;

pub struct LeaseLeasedState {
    pub base: LeaseStateBase,
}

impl LeaseLeasedState {
    pub fn new(lease: ILease, context: Context) -> Result<Self, String> {
        if context.startTime().is_none() {
            return Err(
                "LeaseLeasedState:constructor() error, context.startTime is undefined.".into(),
            );
        }

        if lease.leaseState.as_deref() != Some(LeaseStateType::Leased) {
            return Err(format!(
                "LeaseLeasedState:constructor() error, incoming lease state {:?} is not {}.",
                lease.leaseState,
                LeaseStateType::Leased
            ));
        }
        if lease.leaseStatus.as_deref() != Some(LeaseStatusType::Locked) {
            return Err(format!(
                "LeaseLeasedState:constructor() error, incoming lease status {:?} is not {}.",
                lease.leaseStatus,
                LeaseStatusType::Locked
            ));
        }
        if lease.leaseId.is_none() {
            return Err(format!(
                "LeaseLeasedState:constructor() error, incoming leaseId {:?} should not be undefined.",
                lease.leaseId
            ));
        }
        if let (Some(expire), Some(start)) = (lease.leaseExpireTime, context.startTime()) {
            if start >= expire {
                return Err(format!(
                    "LeaseLeasedState:constructor() error, incoming leaseExpireTime {:?} is not undefined, and smaller than current time {:?}.",
                    expire, start
                ));
            }
        }
        if lease.leaseDurationType.is_none() {
            return Err(format!(
                "LeaseLeasedState:constructor() error, incoming leaseDurationType {:?} is undefined.",
                lease.leaseDurationType
            ));
        }
        if lease.leaseBreakTime.is_some() {
            return Err(format!(
                "LeaseLeasedState:constructor() error, incoming leaseBreakTime {:?} is not undefined.",
                lease.leaseBreakTime
            ));
        }

        Ok(Self {
            base: LeaseStateBase::new(lease, context),
        })
    }
}

impl ILeaseState for LeaseLeasedState {
    fn lease(&self) -> &ILease {
        &self.base.lease
    }

    fn acquire(
        &self,
        duration: i64,
        proposed_lease_id: Option<&str>,
    ) -> Result<Box<dyn ILeaseState>, crate::errors::StorageError> {
        if proposed_lease_id.unwrap_or("") != self.base.lease.leaseId.as_deref().unwrap_or("") {
            return Err(StorageErrorFactory::getLeaseAlreadyPresent(
                self.base.context.contextId().as_deref(),
            ));
        }

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

        if duration == -1 {
            Ok(Box::new(
                LeaseLeasedState::new(
                    ILease {
                        leaseId: Some(lease_id),
                        leaseState: Some(LeaseStateType::Leased.to_string()),
                        leaseStatus: Some(LeaseStatusType::Locked.to_string()),
                        leaseDurationType: Some(LeaseDurationType::Infinite.to_string()),
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
        } else {
            let expire = start + chrono::Duration::seconds(duration);
            Ok(Box::new(
                LeaseLeasedState::new(
                    ILease {
                        leaseId: Some(lease_id),
                        leaseState: Some(LeaseStateType::Leased.to_string()),
                        leaseStatus: Some(LeaseStatusType::Locked.to_string()),
                        leaseDurationType: Some(LeaseDurationType::Fixed.to_string()),
                        leaseDurationSeconds: Some(duration),
                        leaseExpireTime: Some(expire),
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
    }

    /// Mirrors TS `LeaseLeasedState.break()` exactly, including the infinite-lease
    /// `undefined/0` branch.
    fn break_lease(
        &self,
        break_period: Option<i64>,
    ) -> Result<Box<dyn ILeaseState>, crate::errors::StorageError> {
        let start = self.base.context.startTime().unwrap();
        let ctx = self.base.context.clone();
        let lease_id = self.base.lease.leaseId.clone();

        if self.base.lease.leaseDurationType.as_deref() == Some(LeaseDurationType::Infinite) {
            if break_period == Some(0) || break_period.is_none() {
                return Ok(Box::new(
                    crate::lease::lease_broken_state::LeaseBrokenState::new(
                        ILease {
                            leaseId: lease_id,
                            leaseState: Some(LeaseStateType::Broken.to_string()),
                            leaseStatus: Some(LeaseStatusType::Unlocked.to_string()),
                            leaseDurationType: None,
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
                ));
            } else if let Some(bp) = break_period {
                let break_time = start + chrono::Duration::seconds(bp);
                return Ok(Box::new(
                    crate::lease::lease_breaking_state::LeaseBreakingState::new(
                        ILease {
                            leaseId: lease_id,
                            leaseState: Some(LeaseStateType::Breaking.to_string()),
                            leaseStatus: Some(LeaseStatusType::Locked.to_string()),
                            leaseDurationType: None,
                            leaseDurationSeconds: None,
                            leaseExpireTime: None,
                            leaseBreakTime: Some(break_time),
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
                ));
            }
        }

        // Fixed lease follows:
        if break_period == Some(0) {
            return Ok(Box::new(
                crate::lease::lease_broken_state::LeaseBrokenState::new(
                    ILease {
                        leaseId: lease_id,
                        leaseState: Some(LeaseStateType::Broken.to_string()),
                        leaseStatus: Some(LeaseStatusType::Unlocked.to_string()),
                        leaseDurationType: None,
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
            ));
        }

        if break_period.is_none() {
            return Ok(Box::new(
                crate::lease::lease_breaking_state::LeaseBreakingState::new(
                    ILease {
                        leaseId: lease_id,
                        leaseState: Some(LeaseStateType::Breaking.to_string()),
                        leaseStatus: Some(LeaseStatusType::Locked.to_string()),
                        leaseDurationType: None,
                        leaseDurationSeconds: None,
                        leaseExpireTime: None,
                        leaseBreakTime: self.base.lease.leaseExpireTime,
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
            ));
        }

        let bp = break_period.unwrap();
        if !(0..=60).contains(&bp) {
            return Err(StorageErrorFactory::getInvalidLeaseBreakPeriod(
                self.base.context.contextId().as_deref(),
            ));
        }

        // break_period ∈ (0, 60]
        let break_time = start + chrono::Duration::seconds(bp);
        let actual_break_time = minDate(self.base.lease.leaseExpireTime.unwrap(), break_time);
        Ok(Box::new(
            crate::lease::lease_breaking_state::LeaseBreakingState::new(
                ILease {
                    leaseId: lease_id,
                    leaseState: Some(LeaseStateType::Breaking.to_string()),
                    leaseStatus: Some(LeaseStatusType::Locked.to_string()),
                    leaseDurationType: None,
                    leaseDurationSeconds: None,
                    leaseExpireTime: None,
                    leaseBreakTime: Some(actual_break_time),
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

    fn renew(&self, lease_id: &str) -> Result<Box<dyn ILeaseState>, crate::errors::StorageError> {
        if self.base.lease.leaseId.as_deref() != Some(lease_id) {
            return Err(StorageErrorFactory::getLeaseIdMismatchWithLeaseOperation(
                self.base.context.contextId().as_deref(),
            ));
        }

        if self.base.lease.leaseDurationType.as_deref() == Some(LeaseDurationType::Infinite) {
            // Renewing an infinite lease is a no-op — return a new LeaseLeasedState with same data
            return Ok(Box::new(
                LeaseLeasedState::new(self.base.lease.clone(), self.base.context.clone()).map_err(
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
            ));
        }

        let start = self.base.context.startTime().unwrap();
        let duration = self.base.lease.leaseDurationSeconds.unwrap();
        let expire = start + chrono::Duration::seconds(duration);

        Ok(Box::new(
            LeaseLeasedState::new(
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

    fn change(
        &self,
        lease_id: &str,
        proposed_lease_id: &str,
    ) -> Result<Box<dyn ILeaseState>, crate::errors::StorageError> {
        if self.base.lease.leaseId.as_deref() != Some(lease_id)
            && self.base.lease.leaseId.as_deref() != Some(proposed_lease_id)
        {
            return Err(StorageErrorFactory::getLeaseIdMismatchWithLeaseOperation(
                self.base.context.contextId().as_deref(),
            ));
        }

        Ok(Box::new(
            LeaseLeasedState::new(
                ILease {
                    leaseId: Some(proposed_lease_id.to_string()),
                    leaseState: Some(LeaseStateType::Leased.to_string()),
                    leaseStatus: Some(LeaseStatusType::Locked.to_string()),
                    leaseDurationType: self.base.lease.leaseDurationType.clone(),
                    leaseDurationSeconds: self.base.lease.leaseDurationSeconds,
                    leaseExpireTime: self.base.lease.leaseExpireTime,
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
