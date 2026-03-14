/// Mirrors TypeScript `src/blob/lease/BlobWriteLeaseSyncer.ts`.
///
/// Writes the new lease state back to the `BlobModel`, and if the lease
/// transitioned to `Expired` or `Broken` normalises the blob to `Available`
/// (the TODO comment in the TS source is preserved here).
use crate::generated::artifacts::models::GeneratedValue;
use crate::lease::i_lease_state::{ILease, ILeaseSyncer, LeaseStateType, LeaseStatusType};
use crate::persistence::BlobModel;

pub struct BlobWriteLeaseSyncer<'a> {
    pub blob: &'a mut BlobModel,
}

impl<'a> BlobWriteLeaseSyncer<'a> {
    pub fn new(blob: &'a mut BlobModel) -> Self {
        Self { blob }
    }
}

impl<'a> ILeaseSyncer<BlobModel> for BlobWriteLeaseSyncer<'a> {
    /// TODO: Update expire blob lease status to available on write blob operations.
    ///
    /// Need run the function on: PutBlob, SetBlobMetadata, SetBlobProperties,
    /// DeleteBlob, PutBlock, PutBlockList, PutPage, AppendBlock, CopyBlob(dest)
    fn sync(&mut self, lease: &ILease) -> BlobModel {
        // Write the raw lease first
        self.blob.leaseId = lease.leaseId.clone();
        self.blob.leaseExpireTime = lease.leaseExpireTime;
        self.blob.leaseDurationSeconds = lease.leaseDurationSeconds;
        self.blob.leaseBreakTime = lease.leaseBreakTime;

        if let Some(v) = &lease.leaseDurationType {
            self.blob
                .properties
                .insert("leaseDuration".to_string(), GeneratedValue::String(v.clone()));
        } else {
            self.blob.properties.remove("leaseDuration");
        }
        if let Some(v) = &lease.leaseState {
            self.blob
                .properties
                .insert("leaseState".to_string(), GeneratedValue::String(v.clone()));
        } else {
            self.blob.properties.remove("leaseState");
        }
        if let Some(v) = &lease.leaseStatus {
            self.blob
                .properties
                .insert("leaseStatus".to_string(), GeneratedValue::String(v.clone()));
        } else {
            self.blob.properties.remove("leaseStatus");
        }

        let is_expired_or_broken = lease.leaseState.as_deref() == Some(LeaseStateType::Expired)
            || lease.leaseState.as_deref() == Some(LeaseStateType::Broken);

        if is_expired_or_broken {
            self.blob.properties.insert(
                "leaseState".to_string(),
                GeneratedValue::String(LeaseStateType::Available.to_string()),
            );
            self.blob.properties.insert(
                "leaseStatus".to_string(),
                GeneratedValue::String(LeaseStatusType::Unlocked.to_string()),
            );
            self.blob.properties.remove("leaseDuration");
            self.blob.leaseDurationSeconds = None;
            self.blob.leaseId = None;
            self.blob.leaseExpireTime = None;
            self.blob.leaseBreakTime = None;
        } else {
            // Re-write (mirrors the TS `else` branch which sets fields again)
            self.blob.leaseId = lease.leaseId.clone();
            self.blob.leaseExpireTime = lease.leaseExpireTime;
            self.blob.leaseDurationSeconds = lease.leaseDurationSeconds;
            self.blob.leaseBreakTime = lease.leaseBreakTime;

            if let Some(v) = &lease.leaseDurationType {
                self.blob.properties.insert(
                    "leaseDuration".to_string(),
                    GeneratedValue::String(v.clone()),
                );
            } else {
                self.blob.properties.remove("leaseDuration");
            }
            if let Some(v) = &lease.leaseState {
                self.blob.properties.insert(
                    "leaseState".to_string(),
                    GeneratedValue::String(v.clone()),
                );
            } else {
                self.blob.properties.remove("leaseState");
            }
            if let Some(v) = &lease.leaseStatus {
                self.blob.properties.insert(
                    "leaseStatus".to_string(),
                    GeneratedValue::String(v.clone()),
                );
            } else {
                self.blob.properties.remove("leaseStatus");
            }
        }

        self.blob.clone()
    }
}
