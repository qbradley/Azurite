use chrono::{DateTime, Timelike, Utc};

use crate::generated::artifacts::models::GeneratedValue;
use crate::persistence::{BlobModel, ContainerModel, FilterBlobModel};

use super::i_condition_resource::IConditionResource;

/// Mirrors TypeScript `ConditionResourceAdapter` class.
///
/// Represents a resource for conditional header validation.
/// Union of BlobModel | ContainerModel | undefined/null.
pub struct ConditionResourceAdapter;

impl ConditionResourceAdapter {
    /// Build an `IConditionResource` from a `BlobModel`.
    /// Treats uncommitted blobs (isCommitted === false) as nonexistent.
    #[allow(non_snake_case)]
    pub fn from_blob(resource: Option<&BlobModel>) -> IConditionResource {
        match resource {
            None => nonexistent(),
            Some(blob) => {
                // Treat uncommitted blob as nonexistent resource
                let is_committed = blob
                    .properties
                    .get("isCommitted")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true); // default true if field absent (like TS)
                if !blob.isCommitted.unwrap_or(is_committed) {
                    return nonexistent();
                }

                build_from_properties(&blob.properties, || FilterBlobModel {
                    name: blob.name.clone().unwrap_or_default(),
                    containerName: blob.containerName.clone(),
                    tags: blob.blobTags.clone(),
                })
            }
        }
    }

    /// Build an `IConditionResource` from a `ContainerModel`.
    pub fn from_container(resource: Option<&ContainerModel>) -> IConditionResource {
        match resource {
            None => nonexistent(),
            Some(container) => build_from_properties(&container.properties, || FilterBlobModel {
                name: String::new(),
                containerName: container.accountName.clone(),
                tags: None,
            }),
        }
    }
}

fn nonexistent() -> IConditionResource {
    IConditionResource {
        exist: false,
        etag: "NONEXISTENT_RESOURCE_ETAG".to_string(),
        lastModified: None,
        blobItemWithTags: None,
    }
}

#[allow(non_snake_case)]
fn build_from_properties<F>(
    properties: &crate::generated::artifacts::models::GeneratedObject,
    make_filter_blob: F,
) -> IConditionResource
where
    F: FnOnce() -> FilterBlobModel,
{
    let etag = properties
        .get("etag")
        .and_then(|v| v.as_string())
        .unwrap_or_default();

    if etag.len() < 3 {
        // Mirrors TS: throw new Error(`ConditionResourceAdapter::constructor() Invalid etag ${this.etag}.`)
        // In Rust port we panic to preserve the TS error-throw behavior for invalid state.
        panic!(
            "ConditionResourceAdapter::constructor() Invalid etag {}.",
            etag
        );
    }

    let etag = if etag.starts_with('"') && etag.ends_with('"') {
        etag[1..etag.len() - 1].to_string()
    } else {
        etag
    };

    let lastModified = properties
        .get("lastModified")
        .and_then(|v| match v {
            GeneratedValue::String(s) => s.parse::<DateTime<Utc>>().ok(),
            GeneratedValue::Number(n) => DateTime::from_timestamp_millis(*n as i64),
            _ => None,
        })
        .map(|dt| dt.with_nanosecond(0).unwrap_or(dt)); // Precision to seconds

    IConditionResource {
        exist: true,
        etag,
        lastModified,
        blobItemWithTags: Some(make_filter_blob()),
    }
}
