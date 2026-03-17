use azurite_blob::errors::StorageError;
use azurite_blob::generated::artifacts::models::{
    GeneratedObject, GeneratedValue, LeaseAccessConditions,
};
use azurite_blob::generated::context::Context;
use azurite_blob::lease::i_lease_state::{LeaseDurationType, LeaseStateType, LeaseStatusType};
use azurite_blob::lease::lease_available_state::LeaseAvailableState;
use azurite_blob::lease::lease_breaking_state::LeaseBreakingState;
use azurite_blob::lease::lease_broken_state::LeaseBrokenState;
use azurite_blob::lease::lease_expired_state::LeaseExpiredState;
use azurite_blob::lease::lease_leased_state::LeaseLeasedState;
use azurite_blob::lease::{
    BlobLeaseAdapter, BlobLeaseSyncer, BlobReadLeaseValidator, BlobWriteLeaseSyncer,
    BlobWriteLeaseValidator, ContainerLeaseAdapter, ContainerLeaseSyncer, ILease, ILeaseState,
    ILeaseSyncer, ILeaseValidator, LeaseFactory,
};
use azurite_blob::persistence::{BlobModel, ContainerModel};
use chrono::{DateTime, Duration, TimeZone, Utc};
use pretty_assertions::assert_eq;
use uuid::Uuid;

const CONTEXT_ID: &str = "phase8-context";
const LEASE_ID: &str = "lease-123";

fn base_time() -> DateTime<Utc> {
    Utc.timestamp_opt(1_700_000_000, 0).single().unwrap()
}

fn context_at(start_time: Option<DateTime<Utc>>) -> Context {
    let holder = Context::new_holder();
    let context = Context::from_holder(holder, "/blob/phase8", None, None);
    context.setContextId(Some(CONTEXT_ID.to_string()));
    context.setStartTime(start_time);
    context
}

fn lease(
    lease_state: Option<&str>,
    lease_status: Option<&str>,
    lease_id: Option<&str>,
    lease_duration_type: Option<&str>,
    lease_duration_seconds: Option<i64>,
    lease_expire_time: Option<DateTime<Utc>>,
    lease_break_time: Option<DateTime<Utc>>,
) -> ILease {
    ILease {
        leaseId: lease_id.map(str::to_string),
        leaseState: lease_state.map(str::to_string),
        leaseStatus: lease_status.map(str::to_string),
        leaseDurationType: lease_duration_type.map(str::to_string),
        leaseDurationSeconds: lease_duration_seconds,
        leaseExpireTime: lease_expire_time,
        leaseBreakTime: lease_break_time,
    }
}

fn available_lease() -> ILease {
    lease(
        Some(LeaseStateType::Available),
        Some(LeaseStatusType::Unlocked),
        None,
        None,
        None,
        None,
        None,
    )
}

fn fixed_leased_lease(now: DateTime<Utc>, duration_secs: i64, lease_id: &str) -> ILease {
    lease(
        Some(LeaseStateType::Leased),
        Some(LeaseStatusType::Locked),
        Some(lease_id),
        Some(LeaseDurationType::Fixed),
        Some(duration_secs),
        Some(now + Duration::seconds(duration_secs)),
        None,
    )
}

fn infinite_leased_lease(lease_id: &str) -> ILease {
    lease(
        Some(LeaseStateType::Leased),
        Some(LeaseStatusType::Locked),
        Some(lease_id),
        Some(LeaseDurationType::Infinite),
        None,
        None,
        None,
    )
}

fn breaking_lease(now: DateTime<Utc>, lease_id: &str, break_after_secs: i64) -> ILease {
    lease(
        Some(LeaseStateType::Breaking),
        Some(LeaseStatusType::Locked),
        Some(lease_id),
        None,
        None,
        None,
        Some(now + Duration::seconds(break_after_secs)),
    )
}

fn broken_lease(lease_id: &str) -> ILease {
    lease(
        Some(LeaseStateType::Broken),
        Some(LeaseStatusType::Unlocked),
        Some(lease_id),
        None,
        None,
        None,
        None,
    )
}

fn expired_lease(lease_id: &str, duration_secs: i64) -> ILease {
    lease(
        Some(LeaseStateType::Expired),
        Some(LeaseStatusType::Unlocked),
        Some(lease_id),
        None,
        Some(duration_secs),
        None,
        None,
    )
}

fn blob_model() -> BlobModel {
    BlobModel {
        accountName: "devstoreaccount1".to_string(),
        containerName: "container".to_string(),
        ..Default::default()
    }
}

fn container_model() -> ContainerModel {
    ContainerModel {
        accountName: "devstoreaccount1".to_string(),
        ..Default::default()
    }
}

fn lease_access_conditions(lease_id: Option<&str>) -> LeaseAccessConditions {
    let mut conditions = LeaseAccessConditions::new();
    if let Some(lease_id) = lease_id {
        conditions.insert(
            "leaseId".to_string(),
            GeneratedValue::String(lease_id.to_string()),
        );
    }
    conditions
}

fn string_prop(properties: &GeneratedObject, key: &str) -> Option<String> {
    properties.get(key).and_then(GeneratedValue::as_string)
}

fn assert_error_code(error: StorageError, expected_code: &str) {
    assert_eq!(error.storageErrorCode, expected_code);
}

#[test]
fn full_break_lifecycle_matches_ts_state_machine() {
    let now = base_time();
    let context = context_at(Some(now));
    let available = LeaseAvailableState::new(available_lease(), context.clone()).unwrap();

    let leased = available.acquire(30, Some(LEASE_ID)).unwrap();
    assert_eq!(
        leased.lease().leaseState.as_deref(),
        Some(LeaseStateType::Leased)
    );
    assert_eq!(
        leased.lease().leaseStatus.as_deref(),
        Some(LeaseStatusType::Locked)
    );
    assert_eq!(leased.lease().leaseId.as_deref(), Some(LEASE_ID));
    assert_eq!(
        leased.lease().leaseExpireTime,
        Some(now + Duration::seconds(30))
    );

    let breaking = leased.break_lease(Some(15)).unwrap();
    assert_eq!(
        breaking.lease().leaseState.as_deref(),
        Some(LeaseStateType::Breaking)
    );
    assert_eq!(
        breaking.lease().leaseStatus.as_deref(),
        Some(LeaseStatusType::Locked)
    );
    assert_eq!(
        breaking.lease().leaseBreakTime,
        Some(now + Duration::seconds(15))
    );

    let broken = LeaseFactory::createLeaseState(
        breaking.lease().clone(),
        context_at(Some(now + Duration::seconds(16))),
    )
    .unwrap();
    assert_eq!(
        broken.lease().leaseState.as_deref(),
        Some(LeaseStateType::Broken)
    );
    assert_eq!(
        broken.lease().leaseStatus.as_deref(),
        Some(LeaseStatusType::Unlocked)
    );
    assert_eq!(broken.lease().leaseBreakTime, None);

    let reacquired = broken.acquire(30, Some("lease-456")).unwrap();
    assert_eq!(
        reacquired.lease().leaseState.as_deref(),
        Some(LeaseStateType::Leased)
    );
    assert_eq!(reacquired.lease().leaseId.as_deref(), Some("lease-456"));
}

#[test]
fn full_expiry_lifecycle_releases_back_to_available() {
    let now = base_time();
    let expired = LeaseFactory::createLeaseState(
        fixed_leased_lease(now, 30, LEASE_ID),
        context_at(Some(now + Duration::seconds(31))),
    )
    .unwrap();

    assert_eq!(
        expired.lease().leaseState.as_deref(),
        Some(LeaseStateType::Expired)
    );
    assert_eq!(
        expired.lease().leaseStatus.as_deref(),
        Some(LeaseStatusType::Unlocked)
    );
    assert_eq!(expired.lease().leaseId.as_deref(), Some(LEASE_ID));
    assert_eq!(expired.lease().leaseDurationSeconds, Some(30));
    assert_eq!(expired.lease().leaseExpireTime, None);
    assert_eq!(expired.lease().leaseDurationType, None);

    let available = expired.release(LEASE_ID).unwrap();
    assert_eq!(
        available.lease().leaseState.as_deref(),
        Some(LeaseStateType::Available)
    );
    assert_eq!(
        available.lease().leaseStatus.as_deref(),
        Some(LeaseStatusType::Unlocked)
    );
    assert_eq!(available.lease().leaseId, None);
}

#[test]
fn available_acquire_infinite_generates_uuid_when_id_missing() {
    let available =
        LeaseAvailableState::new(available_lease(), context_at(Some(base_time()))).unwrap();

    let leased = available.acquire(-1, None).unwrap();

    assert_eq!(
        leased.lease().leaseState.as_deref(),
        Some(LeaseStateType::Leased)
    );
    assert_eq!(
        leased.lease().leaseStatus.as_deref(),
        Some(LeaseStatusType::Locked)
    );
    assert_eq!(
        leased.lease().leaseDurationType.as_deref(),
        Some(LeaseDurationType::Infinite)
    );
    assert_eq!(leased.lease().leaseDurationSeconds, None);
    assert_eq!(leased.lease().leaseExpireTime, None);
    assert!(Uuid::parse_str(leased.lease().leaseId.as_deref().unwrap()).is_ok());
}

#[test]
fn available_illegal_transitions_use_ts_error_codes() {
    let available =
        LeaseAvailableState::new(available_lease(), context_at(Some(base_time()))).unwrap();

    assert_error_code(
        available.break_lease(None).err().expect("expected error"),
        "LeaseNotPresentWithLeaseOperation",
    );
    assert_error_code(
        available.renew(LEASE_ID).err().expect("expected error"),
        "LeaseIdMismatchWithLeaseOperation",
    );
    assert_error_code(
        available
            .change(LEASE_ID, "next-id")
            .err()
            .expect("expected error"),
        "LeaseNotPresentWithLeaseOperation",
    );
    assert_error_code(
        available.release(LEASE_ID).err().expect("expected error"),
        "LeaseIdMismatchWithLeaseOperation",
    );
}

#[test]
fn leased_break_without_period_uses_original_expiry_deadline() {
    let now = base_time();
    let leased =
        LeaseLeasedState::new(fixed_leased_lease(now, 30, LEASE_ID), context_at(Some(now)))
            .unwrap();

    let breaking = leased.break_lease(None).unwrap();

    assert_eq!(
        breaking.lease().leaseState.as_deref(),
        Some(LeaseStateType::Breaking)
    );
    assert_eq!(
        breaking.lease().leaseBreakTime,
        Some(now + Duration::seconds(30))
    );
}

#[test]
fn leased_break_clamps_break_period_to_earlier_expiry() {
    let now = base_time();
    let leased =
        LeaseLeasedState::new(fixed_leased_lease(now, 20, LEASE_ID), context_at(Some(now)))
            .unwrap();

    let breaking = leased.break_lease(Some(60)).unwrap();

    assert_eq!(
        breaking.lease().leaseState.as_deref(),
        Some(LeaseStateType::Breaking)
    );
    assert_eq!(
        breaking.lease().leaseBreakTime,
        Some(now + Duration::seconds(20))
    );
}

#[test]
fn leased_break_rejects_invalid_break_periods() {
    let now = base_time();
    let leased =
        LeaseLeasedState::new(fixed_leased_lease(now, 30, LEASE_ID), context_at(Some(now)))
            .unwrap();

    assert_error_code(
        leased.break_lease(Some(-1)).err().expect("expected error"),
        "InvalidHeaderValue",
    );
    assert_error_code(
        leased.break_lease(Some(61)).err().expect("expected error"),
        "InvalidHeaderValue",
    );
}

#[test]
fn leased_renew_fixed_resets_expiry_from_current_context_time() {
    let now = base_time();
    let leased = LeaseLeasedState::new(
        fixed_leased_lease(now - Duration::seconds(10), 30, LEASE_ID),
        context_at(Some(now)),
    )
    .unwrap();

    let renewed = leased.renew(LEASE_ID).unwrap();

    assert_eq!(
        renewed.lease().leaseExpireTime,
        Some(now + Duration::seconds(30))
    );
    assert_eq!(
        renewed.lease().leaseDurationType.as_deref(),
        Some(LeaseDurationType::Fixed)
    );
    assert_eq!(renewed.lease().leaseDurationSeconds, Some(30));
}

#[test]
fn leased_renew_infinite_is_a_noop_clone() {
    let leased = LeaseLeasedState::new(
        infinite_leased_lease(LEASE_ID),
        context_at(Some(base_time())),
    )
    .unwrap();

    let renewed = leased.renew(LEASE_ID).unwrap();

    assert_eq!(
        renewed.lease().leaseState.as_deref(),
        Some(LeaseStateType::Leased)
    );
    assert_eq!(
        renewed.lease().leaseDurationType.as_deref(),
        Some(LeaseDurationType::Infinite)
    );
    assert_eq!(renewed.lease().leaseId.as_deref(), Some(LEASE_ID));
    assert_eq!(renewed.lease().leaseExpireTime, None);
}

#[test]
fn leased_change_accepts_matching_proposed_id_and_rewrites_lease_id() {
    let now = base_time();
    let leased =
        LeaseLeasedState::new(fixed_leased_lease(now, 30, LEASE_ID), context_at(Some(now)))
            .unwrap();

    let changed = leased.change(LEASE_ID, "new-lease-id").unwrap();

    assert_eq!(
        changed.lease().leaseState.as_deref(),
        Some(LeaseStateType::Leased)
    );
    assert_eq!(changed.lease().leaseId.as_deref(), Some("new-lease-id"));
    assert_eq!(
        changed.lease().leaseExpireTime,
        Some(now + Duration::seconds(30))
    );
}

#[test]
fn breaking_break_none_returns_same_breaking_state() {
    let now = base_time();
    let breaking =
        LeaseBreakingState::new(breaking_lease(now, LEASE_ID, 30), context_at(Some(now))).unwrap();

    let same_state = breaking.break_lease(None).unwrap();

    assert_eq!(
        same_state.lease().leaseState.as_deref(),
        Some(LeaseStateType::Breaking)
    );
    assert_eq!(
        same_state.lease().leaseBreakTime,
        Some(now + Duration::seconds(30))
    );
}

#[test]
fn breaking_break_shortens_to_earlier_deadline() {
    let now = base_time();
    let breaking =
        LeaseBreakingState::new(breaking_lease(now, LEASE_ID, 30), context_at(Some(now))).unwrap();

    let shortened = breaking.break_lease(Some(10)).unwrap();

    assert_eq!(
        shortened.lease().leaseState.as_deref(),
        Some(LeaseStateType::Breaking)
    );
    assert_eq!(
        shortened.lease().leaseBreakTime,
        Some(now + Duration::seconds(10))
    );
}

#[test]
fn breaking_invalid_transitions_match_ts_parameter_quirks() {
    let now = base_time();
    let breaking =
        LeaseBreakingState::new(breaking_lease(now, LEASE_ID, 30), context_at(Some(now))).unwrap();

    assert_error_code(
        breaking
            .acquire(30, Some(LEASE_ID))
            .err()
            .expect("expected error"),
        "LeaseIsBreakingAndCannotBeAcquired",
    );
    assert_error_code(
        breaking
            .acquire(30, Some("other-id"))
            .err()
            .expect("expected error"),
        "LeaseAlreadyPresent",
    );
    assert_error_code(
        breaking.renew(LEASE_ID).err().expect("expected error"),
        "LeaseIsBrokenAndCannotBeRenewed",
    );
    assert_error_code(
        breaking
            .change(LEASE_ID, "ignored-second-arg")
            .err()
            .expect("expected error"),
        "LeaseIsBreakingAndCannotBeChanged",
    );
    assert_error_code(
        breaking
            .change("wrong-first-arg", LEASE_ID)
            .err()
            .expect("expected error"),
        "LeaseIdMismatchWithLeaseOperation",
    );
}

#[test]
fn broken_break_is_noop_and_other_invalid_transitions_keep_ts_errors() {
    let broken =
        LeaseBrokenState::new(broken_lease(LEASE_ID), context_at(Some(base_time()))).unwrap();

    let same_state = broken.break_lease(Some(30)).unwrap();
    assert_eq!(
        same_state.lease().leaseState.as_deref(),
        Some(LeaseStateType::Broken)
    );
    assert_eq!(same_state.lease().leaseId.as_deref(), Some(LEASE_ID));

    assert_error_code(
        broken.renew(LEASE_ID).err().expect("expected error"),
        "LeaseIsBrokenAndCannotBeRenewed",
    );
    assert_error_code(
        broken
            .change(LEASE_ID, "new-id")
            .err()
            .expect("expected error"),
        "LeaseNotPresentWithLeaseOperation",
    );
}

#[test]
fn expired_renew_ignores_caller_lease_id() {
    let now = base_time();
    let expired =
        LeaseExpiredState::new(expired_lease(LEASE_ID, 30), context_at(Some(now))).unwrap();

    let renewed = expired.renew("totally-wrong-id").unwrap();

    assert_eq!(
        renewed.lease().leaseState.as_deref(),
        Some(LeaseStateType::Leased)
    );
    assert_eq!(renewed.lease().leaseId.as_deref(), Some(LEASE_ID));
    assert_eq!(
        renewed.lease().leaseExpireTime,
        Some(now + Duration::seconds(30))
    );
}

#[test]
fn expired_break_goes_directly_to_broken() {
    let expired =
        LeaseExpiredState::new(expired_lease(LEASE_ID, 30), context_at(Some(base_time()))).unwrap();

    let broken = expired.break_lease(Some(15)).unwrap();

    assert_eq!(
        broken.lease().leaseState.as_deref(),
        Some(LeaseStateType::Broken)
    );
    assert_eq!(
        broken.lease().leaseStatus.as_deref(),
        Some(LeaseStatusType::Unlocked)
    );
    assert_eq!(broken.lease().leaseId.as_deref(), Some(LEASE_ID));
}

#[test]
fn lease_factory_treats_missing_state_as_available_without_eager_timer() {
    let state =
        LeaseFactory::createLeaseState(ILease::default(), context_at(Some(base_time()))).unwrap();

    assert_eq!(state.lease().leaseState, None);
    assert_eq!(state.lease().leaseStatus, None);

    let leased = state.acquire(30, Some(LEASE_ID)).unwrap();
    assert_eq!(
        leased.lease().leaseState.as_deref(),
        Some(LeaseStateType::Leased)
    );
}

#[test]
fn lease_factory_requires_context_start_time() {
    let error = LeaseFactory::createLeaseState(available_lease(), context_at(None))
        .err()
        .expect("expected error");
    assert_error_code(error, "InternalError");
}

#[test]
fn blob_lease_adapter_defaults_missing_state_and_status_but_preserves_fields() {
    let now = base_time();
    let mut blob = blob_model();
    blob.leaseId = Some(LEASE_ID.to_string());
    blob.leaseDurationSeconds = Some(30);
    blob.leaseExpireTime = Some(now + Duration::seconds(30));
    blob.leaseBreakTime = Some(now + Duration::seconds(10));
    blob.properties.insert(
        "leaseDuration".to_string(),
        GeneratedValue::String(LeaseDurationType::Fixed.to_string()),
    );

    let adapted = BlobLeaseAdapter::from_blob(&mut blob);

    assert_eq!(adapted.leaseId.as_deref(), Some(LEASE_ID));
    assert_eq!(
        adapted.leaseState.as_deref(),
        Some(LeaseStateType::Available)
    );
    assert_eq!(
        adapted.leaseStatus.as_deref(),
        Some(LeaseStatusType::Unlocked)
    );
    assert_eq!(
        adapted.leaseDurationType.as_deref(),
        Some(LeaseDurationType::Fixed)
    );
    assert_eq!(adapted.leaseDurationSeconds, Some(30));
    assert_eq!(adapted.leaseExpireTime, Some(now + Duration::seconds(30)));
    assert_eq!(adapted.leaseBreakTime, Some(now + Duration::seconds(10)));
    assert_eq!(
        string_prop(&blob.properties, "leaseState").as_deref(),
        Some(LeaseStateType::Available)
    );
    assert_eq!(
        string_prop(&blob.properties, "leaseStatus").as_deref(),
        Some(LeaseStatusType::Unlocked)
    );
}

#[test]
fn container_lease_adapter_wraps_existing_fields_and_rejects_missing_state() {
    let now = base_time();
    let mut container = container_model();
    container.leaseId = Some(LEASE_ID.to_string());
    container.leaseDurationSeconds = Some(30);
    container.leaseExpireTime = Some(now + Duration::seconds(30));
    container.leaseBreakTime = Some(now + Duration::seconds(10));
    container.properties.insert(
        "leaseState".to_string(),
        GeneratedValue::String(LeaseStateType::Breaking.to_string()),
    );
    container.properties.insert(
        "leaseStatus".to_string(),
        GeneratedValue::String(LeaseStatusType::Locked.to_string()),
    );
    container.properties.insert(
        "leaseDuration".to_string(),
        GeneratedValue::String(LeaseDurationType::Fixed.to_string()),
    );

    let adapted = ContainerLeaseAdapter::from_container(&container).unwrap();
    assert_eq!(adapted.leaseId.as_deref(), Some(LEASE_ID));
    assert_eq!(
        adapted.leaseState.as_deref(),
        Some(LeaseStateType::Breaking)
    );
    assert_eq!(
        adapted.leaseStatus.as_deref(),
        Some(LeaseStatusType::Locked)
    );
    assert_eq!(
        adapted.leaseDurationType.as_deref(),
        Some(LeaseDurationType::Fixed)
    );
    assert_eq!(adapted.leaseExpireTime, Some(now + Duration::seconds(30)));
    assert_eq!(adapted.leaseBreakTime, Some(now + Duration::seconds(10)));

    let missing =
        ContainerLeaseAdapter::from_container(&container_model()).expect_err("expected error");
    assert_eq!(
        missing,
        "ContainerLeaseAdapter:constructor() container leaseState cannot be undefined."
    );
}

#[test]
fn blob_and_container_syncers_copy_lease_fields() {
    let now = base_time();
    let lease = fixed_leased_lease(now, 30, LEASE_ID);

    let synced_blob = {
        let mut blob = blob_model();
        let mut syncer = BlobLeaseSyncer::new(&mut blob);
        syncer.sync(&lease)
    };
    assert_eq!(synced_blob.leaseId.as_deref(), Some(LEASE_ID));
    assert_eq!(synced_blob.leaseDurationSeconds, Some(30));
    assert_eq!(
        synced_blob.leaseExpireTime,
        Some(now + Duration::seconds(30))
    );
    assert_eq!(
        string_prop(&synced_blob.properties, "leaseDuration").as_deref(),
        Some(LeaseDurationType::Fixed)
    );
    assert_eq!(
        string_prop(&synced_blob.properties, "leaseState").as_deref(),
        Some(LeaseStateType::Leased)
    );
    assert_eq!(
        string_prop(&synced_blob.properties, "leaseStatus").as_deref(),
        Some(LeaseStatusType::Locked)
    );

    let synced_container = {
        let mut container = container_model();
        let mut syncer = ContainerLeaseSyncer::new(&mut container);
        syncer.sync(&lease)
    };
    assert_eq!(synced_container.leaseId.as_deref(), Some(LEASE_ID));
    assert_eq!(synced_container.leaseDurationSeconds, Some(30));
    assert_eq!(
        synced_container.leaseExpireTime,
        Some(now + Duration::seconds(30))
    );
    assert_eq!(
        string_prop(&synced_container.properties, "leaseDuration").as_deref(),
        Some(LeaseDurationType::Fixed)
    );
    assert_eq!(
        string_prop(&synced_container.properties, "leaseState").as_deref(),
        Some(LeaseStateType::Leased)
    );
    assert_eq!(
        string_prop(&synced_container.properties, "leaseStatus").as_deref(),
        Some(LeaseStatusType::Locked)
    );
}

#[test]
fn blob_write_syncer_normalizes_expired_and_broken_to_available() {
    let expired = expired_lease(LEASE_ID, 30);
    let expired_blob = {
        let mut blob = blob_model();
        let mut syncer = BlobWriteLeaseSyncer::new(&mut blob);
        syncer.sync(&expired)
    };
    assert_eq!(expired_blob.leaseId, None);
    assert_eq!(expired_blob.leaseDurationSeconds, None);
    assert_eq!(expired_blob.leaseExpireTime, None);
    assert_eq!(
        string_prop(&expired_blob.properties, "leaseState").as_deref(),
        Some(LeaseStateType::Available)
    );
    assert_eq!(
        string_prop(&expired_blob.properties, "leaseStatus").as_deref(),
        Some(LeaseStatusType::Unlocked)
    );
    assert_eq!(string_prop(&expired_blob.properties, "leaseDuration"), None);

    let broken_blob = {
        let mut blob = blob_model();
        let mut syncer = BlobWriteLeaseSyncer::new(&mut blob);
        syncer.sync(&broken_lease(LEASE_ID))
    };
    assert_eq!(broken_blob.leaseId, None);
    assert_eq!(
        string_prop(&broken_blob.properties, "leaseState").as_deref(),
        Some(LeaseStateType::Available)
    );
    assert_eq!(
        string_prop(&broken_blob.properties, "leaseStatus").as_deref(),
        Some(LeaseStatusType::Unlocked)
    );
}

#[test]
fn blob_read_validator_is_permissive_without_input_id_and_reports_stale_unlocked_leases() {
    let context = context_at(Some(base_time()));
    let unlocked = available_lease();

    BlobReadLeaseValidator::new(None)
        .validate(&unlocked, &context)
        .unwrap();
    BlobReadLeaseValidator::new(Some(lease_access_conditions(Some(""))))
        .validate(&unlocked, &context)
        .unwrap();

    let error = BlobReadLeaseValidator::new(Some(lease_access_conditions(Some(LEASE_ID))))
        .validate(&unlocked, &context)
        .expect_err("expected error");
    assert_error_code(error, "LeaseNotPresentWithBlobOperation");
}

#[test]
fn blob_write_validator_requires_id_when_locked_and_rejects_stale_ids() {
    let context = context_at(Some(base_time()));
    let locked = fixed_leased_lease(base_time(), 30, LEASE_ID);
    let unlocked = available_lease();

    let missing = BlobWriteLeaseValidator::new(None)
        .validate(&locked, &context)
        .expect_err("expected error");
    assert_error_code(missing, "LeaseIdMissing");

    let mismatch = BlobWriteLeaseValidator::new(Some(lease_access_conditions(Some("other-id"))))
        .validate(&locked, &context)
        .expect_err("expected error");
    assert_error_code(mismatch, "LeaseIdMismatchWithBlobOperation");

    let stale = BlobWriteLeaseValidator::new(Some(lease_access_conditions(Some(LEASE_ID))))
        .validate(&unlocked, &context)
        .expect_err("expected error");
    assert_error_code(stale, "LeaseNotPresentWithBlobOperation");
}

#[test]
fn blob_validators_match_lease_ids_case_insensitively() {
    let context = context_at(Some(base_time()));
    let locked = fixed_leased_lease(base_time(), 30, "Lease-ABC");
    let conditions = Some(lease_access_conditions(Some("lease-abc")));

    BlobReadLeaseValidator::new(conditions.clone())
        .validate(&locked, &context)
        .unwrap();
    BlobWriteLeaseValidator::new(conditions)
        .validate(&locked, &context)
        .unwrap();
}
