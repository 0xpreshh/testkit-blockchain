//! Integration tests against `examples/vault`, run as a subprocess of the
//! real `soroban-testkit` binary — exercising the CLI the way a user
//! would, not just its internal functions.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_soroban-testkit"))
        .args(args)
        .current_dir(workspace_root())
        .output()
        .expect("failed to run soroban-testkit")
}

fn run_with_stdin(args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_soroban-testkit"))
        .args(args)
        .current_dir(workspace_root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to run soroban-testkit");
    child
        .stdin
        .take()
        .expect("child stdin")
        .write_all(stdin.as_bytes())
        .expect("failed to write stdin");
    child
        .wait_with_output()
        .expect("failed to wait on soroban-testkit")
}

#[test]
fn audit_reports_findings_against_vault() {
    let output = run(&["audit", "examples/vault/src"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("missing-require-auth"), "{stdout}");
    assert!(stdout.contains("missing-ttl-bump"), "{stdout}");
    assert!(stdout.contains("unchecked-i128-arithmetic"), "{stdout}");
    assert!(stdout.contains("not a security product"), "{stdout}");
}

#[test]
fn audit_strict_exits_non_zero_on_findings() {
    let output = run(&["audit", "examples/vault/src", "--strict"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("audit finding(s)"), "{stderr}");
}

#[test]
fn audit_reports_no_findings_on_a_clean_directory() {
    let output = run(&["audit", "crates/cli/src/commands/mod.rs"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("no findings"), "{stdout}");
}

#[test]
fn audit_reads_source_from_stdin_when_path_is_dash() {
    let src = r#"
        fn transfer(to: Address, amount: i128) {
            let total = amount + 1;
        }
    "#;
    let output = run_with_stdin(&["audit", "-"], src);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("missing-require-auth"), "{stdout}");
    assert!(stdout.contains("unchecked-i128-arithmetic"), "{stdout}");
    assert!(stdout.contains("<stdin>"), "{stdout}");
}

#[test]
fn audit_stdin_reports_no_findings_on_empty_input() {
    let output = run_with_stdin(&["audit", "-"], "");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("no findings"), "{stdout}");
}

#[test]
fn audit_stdin_reports_no_findings_on_invalid_input() {
    let output = run_with_stdin(&["audit", "-"], "fn {{{ not valid rust");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("no findings"), "{stdout}");
}

#[test]
fn limits_finds_the_real_mainnet_write_ceiling_for_batch_payout() {
    let build = Command::new("cargo")
        .args([
            "build",
            "-p",
            "vault",
            "--target",
            "wasm32v1-none",
            "--release",
        ])
        .current_dir(workspace_root())
        .status()
        .expect("failed to build vault.wasm");
    assert!(build.success());

    let wasm = workspace_root().join("target/wasm32v1-none/release/vault.wasm");
    let output = run(&[
        "limits",
        "--contract",
        wasm.to_str().unwrap(),
        "--fn",
        "batch_payout",
        "--ramp",
        "recipients",
    ]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("last successful recipients"), "{stdout}");
    assert!(stdout.contains("instructions:"), "{stdout}");
    assert!(stdout.contains("memory bytes:"), "{stdout}");
}

#[test]
fn limits_reports_an_actionable_error_for_an_unknown_function() {
    let wasm = workspace_root().join("target/wasm32v1-none/release/vault.wasm");
    if !wasm.exists() {
        // Built by limits_finds_the_real_mainnet_write_ceiling_for_batch_payout;
        // skip if tests ran in isolation without it.
        return;
    }
    let output = run(&[
        "limits",
        "--contract",
        wasm.to_str().unwrap(),
        "--fn",
        "does_not_exist",
        "--ramp",
        "x",
    ]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains("no function named"), "{stderr}");
}
