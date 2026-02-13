use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

fn hemli_cmd() -> Command {
    cargo_bin_cmd!("hemli")
}

#[test]
fn test_help() {
    hemli_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Secret management CLI"));
}

#[test]
fn test_get_subcommand_help() {
    hemli_cmd()
        .args(["get", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--namespace"))
        .stdout(predicate::str::contains("--source-sh"))
        .stdout(predicate::str::contains("--source-cmd"))
        .stdout(predicate::str::contains("--ttl"))
        .stdout(predicate::str::contains("--force-refresh"))
        .stdout(predicate::str::contains("--no-refresh"))
        .stdout(predicate::str::contains("--no-store"));
}

#[test]
fn test_missing_namespace_errors() {
    hemli_cmd()
        .args(["get", "mysecret"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--namespace"));
}

#[test]
fn test_source_sh_and_source_cmd_conflict() {
    hemli_cmd()
        .args([
            "get",
            "-n",
            "ns",
            "sec",
            "--source-sh",
            "echo hi",
            "--source-cmd",
            "echo hi",
        ])
        .assert()
        .failure();
}

#[test]
fn test_force_refresh_and_no_refresh_conflict() {
    hemli_cmd()
        .args(["get", "-n", "ns", "sec", "--force-refresh", "--no-refresh"])
        .assert()
        .failure();
}

fn test_namespace() -> String {
    format!("hemli-e2e-test-{}", std::process::id())
}

fn cleanup(namespace: &str, secret: &str) {
    let _ = hemli_cmd()
        .args(["delete", "-n", namespace, secret])
        .output();
}

#[test]
#[ignore] // Requires OS keyring access
fn test_get_store_and_retrieve() {
    let ns = test_namespace();
    let secret = "test-store-retrieve";
    cleanup(&ns, secret);

    // First get: fetch from source
    hemli_cmd()
        .args(["get", "-n", &ns, secret, "--source-sh", "echo hello-e2e"])
        .assert()
        .success()
        .stdout("hello-e2e");

    // Second get: should return cached value (no source needed)
    hemli_cmd()
        .args(["get", "-n", &ns, secret])
        .assert()
        .success()
        .stdout("hello-e2e");

    cleanup(&ns, secret);
}

#[test]
#[ignore] // Requires OS keyring access
fn test_no_refresh_fails_when_not_stored() {
    let ns = test_namespace();
    let secret = "test-no-refresh-missing";
    cleanup(&ns, secret);

    hemli_cmd()
        .args(["get", "-n", &ns, secret, "--no-refresh"])
        .assert()
        .failure();

    cleanup(&ns, secret);
}

#[test]
#[ignore] // Requires OS keyring access
fn test_no_store_does_not_persist() {
    let ns = test_namespace();
    let secret = "test-no-store";
    cleanup(&ns, secret);

    // Get with --no-store
    hemli_cmd()
        .args([
            "get",
            "-n",
            &ns,
            secret,
            "--source-sh",
            "echo ephemeral",
            "--no-store",
        ])
        .assert()
        .success()
        .stdout("ephemeral");

    // Should not be cached
    hemli_cmd()
        .args(["get", "-n", &ns, secret, "--no-refresh"])
        .assert()
        .failure();

    cleanup(&ns, secret);
}

#[test]
#[ignore] // Requires OS keyring access
fn test_force_refresh() {
    let ns = test_namespace();
    let secret = "test-force-refresh";
    cleanup(&ns, secret);

    // Store initial value
    hemli_cmd()
        .args(["get", "-n", &ns, secret, "--source-sh", "echo old-value"])
        .assert()
        .success()
        .stdout("old-value");

    // Force refresh with new value
    hemli_cmd()
        .args([
            "get",
            "-n",
            &ns,
            secret,
            "--force-refresh",
            "--source-sh",
            "echo new-value",
        ])
        .assert()
        .success()
        .stdout("new-value");

    // Cached value should be the new one
    hemli_cmd()
        .args(["get", "-n", &ns, secret, "--no-refresh"])
        .assert()
        .success()
        .stdout("new-value");

    cleanup(&ns, secret);
}

#[test]
#[ignore] // Requires OS keyring access
fn test_list_shows_stored_secrets() {
    let ns = test_namespace();
    let secret = "test-list-secret";
    cleanup(&ns, secret);

    hemli_cmd()
        .args(["get", "-n", &ns, secret, "--source-sh", "echo listed"])
        .assert()
        .success();

    hemli_cmd()
        .args(["list", "-n", &ns])
        .assert()
        .success()
        .stdout(predicate::str::contains(&ns))
        .stdout(predicate::str::contains(secret));

    cleanup(&ns, secret);
}

#[test]
#[ignore] // Requires OS keyring access
fn test_delete_nonexistent_succeeds() {
    let ns = test_namespace();
    hemli_cmd()
        .args(["delete", "-n", &ns, "nonexistent-secret"])
        .assert()
        .success();
}
