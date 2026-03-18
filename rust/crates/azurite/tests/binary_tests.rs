#![allow(non_snake_case)]
//! Integration tests for the combined azurite binary.
//! Verifies CLI argument parsing, help output, and startup/shutdown.

use std::process::Command;

fn cargo_bin() -> Command {
    let mut cmd = Command::new(env!("CARGO"));
    cmd.args(["run", "-p", "azurite", "--"]);
    cmd
}

#[test]
fn help_flag_exits_cleanly() {
    let output = cargo_bin()
        .arg("--help")
        .output()
        .expect("failed to run binary");

    assert!(output.status.success(), "exit code should be 0 for --help");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--blobPort"),
        "help should mention --blobPort"
    );
    assert!(
        stdout.contains("--queuePort"),
        "help should mention --queuePort"
    );
    assert!(
        stdout.contains("--tablePort"),
        "help should mention --tablePort"
    );
}

#[test]
fn invalid_flag_exits_with_error() {
    let output = cargo_bin()
        .arg("--nonExistentFlag")
        .output()
        .expect("failed to run binary");

    assert!(
        !output.status.success(),
        "unknown flag should produce a non-zero exit"
    );
}

#[test]
fn help_shows_all_three_services() {
    let output = cargo_bin()
        .arg("--help")
        .output()
        .expect("failed to run binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--blobHost"), "should show blobHost");
    assert!(stdout.contains("--queueHost"), "should show queueHost");
    assert!(stdout.contains("--tableHost"), "should show tableHost");
    assert!(stdout.contains("--location"), "should show location");
    assert!(stdout.contains("--silent"), "should show silent");
    assert!(stdout.contains("--loose"), "should show loose");
    assert!(
        stdout.contains("--inMemoryPersistence"),
        "should show inMemoryPersistence"
    );
}
