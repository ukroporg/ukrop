use std::process::Command;

fn ukrop_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ukrop"))
}

#[test]
fn test_help() {
    let output = ukrop_bin().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Quick directory jumping"));
}

#[test]
fn test_init_bash() {
    let output = ukrop_bin().args(["init", "bash"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PROMPT_COMMAND"));
    assert!(stdout.contains("__ukrop_hook"));
}

#[test]
fn test_init_zsh() {
    let output = ukrop_bin().args(["init", "zsh"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("precmd_functions"));
    assert!(stdout.contains("__ukrop_hook"));
}

#[test]
fn test_hook_and_list() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.db");
    let db_str = db.to_str().unwrap();

    // Hook a few directories
    let output = ukrop_bin()
        .args(["hook", "--", "/tmp/test1"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    assert!(output.status.success());

    let output = ukrop_bin()
        .args(["hook", "--", "/tmp/test2"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    assert!(output.status.success());

    // List should show them
    let output = ukrop_bin()
        .args(["list"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("/tmp/test1"));
    assert!(stdout.contains("/tmp/test2"));
}

#[test]
fn test_list_json() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.db");
    let db_str = db.to_str().unwrap();

    ukrop_bin()
        .args(["hook", "--", "/tmp/jsontest"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();

    let output = ukrop_bin()
        .args(["list", "--json"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_array());
}

#[test]
fn test_forget() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.db");
    let db_str = db.to_str().unwrap();

    ukrop_bin()
        .args(["hook", "--", "/tmp/forget_me"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();

    let output = ukrop_bin()
        .args(["forget", "/tmp/forget_me"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    assert!(output.status.success());

    let output = ukrop_bin()
        .args(["list"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("/tmp/forget_me"));
}

#[test]
fn test_hook_ssh_and_list() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.db");
    let db_str = db.to_str().unwrap();

    // Record an SSH host via hook
    let output = ukrop_bin()
        .args(["hook-ssh", "--host", "root@myserver.com"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    assert!(output.status.success());

    // Record another with port
    let output = ukrop_bin()
        .args(["hook-ssh", "--host", "-p 2222 admin@db.local"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    assert!(output.status.success());

    // List should show them
    let output = ukrop_bin()
        .args(["list", "--ssh"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("root@myserver.com"));
    assert!(stdout.contains("-p 2222 admin@db.local"));
}

#[test]
fn test_list_ssh_json() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.db");
    let db_str = db.to_str().unwrap();

    ukrop_bin()
        .args(["hook-ssh", "--host", "user@host.example"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();

    let output = ukrop_bin()
        .args(["list", "--ssh", "--json"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_array());
    assert!(parsed.as_array().unwrap().len() > 0);
}

#[test]
fn test_forget_ssh_host() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.db");
    let db_str = db.to_str().unwrap();

    ukrop_bin()
        .args(["hook-ssh", "--host", "forget-me@host"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();

    let output = ukrop_bin()
        .args(["forget", "forget-me@host"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    assert!(output.status.success());

    let output = ukrop_bin()
        .args(["list", "--ssh"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("forget-me@host"));
}

#[test]
fn test_init_fish() {
    let output = ukrop_bin().args(["init", "fish"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("__ukrop_hook"));
    assert!(stdout.contains("fish_postexec"));
}

#[test]
fn test_list_commands() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.db");
    let db_str = db.to_str().unwrap();

    ukrop_bin()
        .args(["hook-cmd", "--cmd", "echo hello", "--exit-code", "0", "--cwd", "/tmp"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();

    let output = ukrop_bin()
        .args(["list", "--commands"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("echo hello"));
}

#[test]
fn test_add_nonexistent_directory() {
    let output = ukrop_bin()
        .args(["add", "/nonexistent/path/that/does/not/exist"])
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn test_export_import_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.db");
    let db_str = db.to_str().unwrap();
    let export_file = dir.path().join("export.jsonl");
    let export_str = export_file.to_str().unwrap();

    // Create some data
    ukrop_bin()
        .args(["hook", "--", "/tmp/export_test1"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    ukrop_bin()
        .args(["hook-cmd", "--cmd", "echo roundtrip", "--exit-code", "0", "--cwd", "/tmp"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    ukrop_bin()
        .args(["hook-ssh", "--host", "roundtrip-host"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();

    // Export
    let output = ukrop_bin()
        .args(["export", "--file", export_str])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Exported"));

    // Verify export file has content
    let content = std::fs::read_to_string(&export_file).unwrap();
    assert!(content.contains("export_test1"));
    assert!(content.contains("echo roundtrip"));
    assert!(content.contains("roundtrip-host"));

    // Import into a fresh DB
    let db2 = dir.path().join("test2.db");
    let db2_str = db2.to_str().unwrap();

    let output = ukrop_bin()
        .args(["import", "--file", export_str])
        .env("UKROP_DB_PATH", db2_str)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Restored"));

    // Verify imported data
    let output = ukrop_bin()
        .args(["list"])
        .env("UKROP_DB_PATH", db2_str)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("export_test1"));

    let output = ukrop_bin()
        .args(["list", "--commands"])
        .env("UKROP_DB_PATH", db2_str)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("echo roundtrip"));

    let output = ukrop_bin()
        .args(["list", "--ssh"])
        .env("UKROP_DB_PATH", db2_str)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("roundtrip-host"));
}

#[test]
fn test_export_to_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.db");
    let db_str = db.to_str().unwrap();

    ukrop_bin()
        .args(["hook", "--", "/tmp/stdout_test"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();

    let output = ukrop_bin()
        .args(["export"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("directory"));
    assert!(stdout.contains("stdout_test"));
}

#[test]
fn test_demo() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.db");
    let db_str = db.to_str().unwrap();

    let output = ukrop_bin()
        .args(["demo"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Generated demo data"));

    // Check directories
    let output = ukrop_bin()
        .args(["list", "--json"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.as_array().unwrap().len() >= 20);

    // Check commands
    let output = ukrop_bin()
        .args(["list", "--commands", "--json"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.as_array().unwrap().len() >= 30);

    // Check SSH hosts
    let output = ukrop_bin()
        .args(["list", "--ssh", "--json"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.as_array().unwrap().len() >= 8);
}

#[test]
fn test_demo_replaces_existing_data() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.db");
    let db_str = db.to_str().unwrap();

    // Add personal data
    ukrop_bin()
        .args(["hook", "--", "/tmp/personal_secret_dir"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();

    // Run demo (should replace)
    ukrop_bin()
        .args(["demo"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();

    // Personal data should be gone
    let output = ukrop_bin()
        .args(["list"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("personal_secret_dir"));
}

#[test]
fn test_forget_nonexistent_entry() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.db");
    let db_str = db.to_str().unwrap();

    let output = ukrop_bin()
        .args(["forget", "/nonexistent/path/12345"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Not found"));
}
