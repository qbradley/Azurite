use std::str::FromStr;

use assert_matches::assert_matches;
use azurite_common::{i_logger_strategy::LogLevels, models::OAuthLevel};
use pretty_assertions::assert_eq;

#[test]
fn oauth_level_parses_basic_case_insensitively() {
    assert_matches!(OAuthLevel::from_str("basic"), Ok(OAuthLevel::BASIC));
    assert_matches!(OAuthLevel::from_str("BASIC"), Ok(OAuthLevel::BASIC));
}

#[test]
fn oauth_level_rejects_unknown_values() {
    let error = OAuthLevel::from_str("advanced").unwrap_err();

    assert_eq!(error.message, "Invalid header value: advanced");
}

#[test]
fn log_levels_preserve_winston_wire_strings() {
    let expected = [
        (LogLevels::Error, "error"),
        (LogLevels::Warn, "warn"),
        (LogLevels::Info, "info"),
        (LogLevels::Verbose, "verbose"),
        (LogLevels::Debug, "debug"),
    ];

    for (level, expected_text) in expected {
        assert_eq!(level.as_str(), expected_text);
    }
}
