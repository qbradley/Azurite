use chrono::{DateTime, Timelike, Utc};

use crate::generated::artifacts::models::{GeneratedValue, ModifiedAccessConditions};
use crate::generated::context::Context;

use super::i_conditional_headers::IConditionalHeaders;

/// Mirrors TypeScript `ConditionalHeadersAdapter` class.
///
/// Adapts `ModifiedAccessConditions` from the generated models into
/// `IConditionalHeaders` by parsing etag lists and truncating date
/// precision to seconds.
#[allow(non_snake_case)]
pub struct ConditionalHeadersAdapter;

impl ConditionalHeadersAdapter {
    /// Construct an `IConditionalHeaders` from a `ModifiedAccessConditions`.
    ///
    /// Mirrors the TypeScript constructor logic exactly:
    /// - splits If-Match / If-None-Match by comma
    /// - strips surrounding quotes from each etag
    /// - truncates milliseconds on date conditions
    pub fn new(
        _context: &Context,
        modified_access_conditions: &ModifiedAccessConditions,
    ) -> IConditionalHeaders {
        let mut headers = IConditionalHeaders::default();

        // If-Match: split by comma, strip surrounding quotes
        if let Some(GeneratedValue::String(if_match)) = modified_access_conditions.get("ifMatch") {
            headers.ifMatch = Some(
                if_match
                    .split(',')
                    .map(|etag| {
                        let etag = etag.trim();
                        if etag.starts_with('"') && etag.ends_with('"') {
                            etag[1..etag.len() - 1].to_string()
                        } else {
                            etag.to_string()
                        }
                    })
                    .collect(),
            );
        }

        // If-None-Match: split by comma, strip surrounding quotes
        if let Some(GeneratedValue::String(if_none_match)) =
            modified_access_conditions.get("ifNoneMatch")
        {
            headers.ifNoneMatch = Some(
                if_none_match
                    .split(',')
                    .map(|etag| {
                        let etag = etag.trim();
                        if etag.starts_with('"') && etag.ends_with('"') {
                            etag[1..etag.len() - 1].to_string()
                        } else {
                            etag.to_string()
                        }
                    })
                    .collect(),
            );
        }

        // If-Modified-Since: parse and truncate milliseconds
        if let Some(GeneratedValue::String(if_modified_since)) =
            modified_access_conditions.get("ifModifiedSince")
        {
            if let Ok(dt) = if_modified_since.parse::<DateTime<Utc>>() {
                headers.ifModifiedSince = Some(truncate_millis(dt));
            }
        }

        // If-Unmodified-Since: parse and truncate milliseconds
        if let Some(GeneratedValue::String(if_unmodified_since)) =
            modified_access_conditions.get("ifUnmodifiedSince")
        {
            if let Ok(dt) = if_unmodified_since.parse::<DateTime<Utc>>() {
                headers.ifUnmodifiedSince = Some(truncate_millis(dt));
            }
        }

        // ifTags
        if let Some(GeneratedValue::String(if_tags)) = modified_access_conditions.get("ifTags") {
            headers.ifTags = Some(if_tags.clone());
        }

        headers
    }
}

/// Truncate milliseconds to match TypeScript `setMilliseconds(0)` behavior.
fn truncate_millis(dt: DateTime<Utc>) -> DateTime<Utc> {
    dt.with_nanosecond(0).unwrap_or(dt)
}
