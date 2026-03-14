use chrono::{DateTime, NaiveDateTime, Timelike, Utc};

use crate::generated::artifacts::models::{GeneratedValue, ModifiedAccessConditions};
use crate::generated::context::Context;

use super::i_conditional_headers::IConditionalHeaders;

/// Parse a date string that may be in ISO 8601, RFC 2822, or HTTP date format.
fn parse_date(s: &str) -> Option<DateTime<Utc>> {
    // Try ISO 8601 (e.g., "2026-03-14T19:43:32Z")
    if let Ok(dt) = s.parse::<DateTime<Utc>>() {
        return Some(dt);
    }
    // Try RFC 2822 (e.g., "Fri, 14 Mar 2026 19:43:32 GMT")
    if let Ok(dt) = DateTime::parse_from_rfc2822(s) {
        return Some(dt.with_timezone(&Utc));
    }
    // Try common HTTP date format (e.g., "Fri, 14 Mar 2026 19:43:32 GMT")
    if let Ok(dt) =
        NaiveDateTime::parse_from_str(s.trim_end_matches(" GMT"), "%a, %d %b %Y %H:%M:%S")
    {
        return Some(dt.and_utc());
    }
    None
}

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
            if let Some(dt) = parse_date(if_modified_since) {
                headers.ifModifiedSince = Some(truncate_millis(dt));
            }
        }

        // If-Unmodified-Since: parse and truncate milliseconds
        if let Some(GeneratedValue::String(if_unmodified_since)) =
            modified_access_conditions.get("ifUnmodifiedSince")
        {
            if let Some(dt) = parse_date(if_unmodified_since) {
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
