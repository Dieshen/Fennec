use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_fennec_help() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "fennec", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Fennec"));
}

#[test]
fn test_fennec_version() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "fennec", "--", "--version"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
}

#[test]
#[ignore = "requires environment setup"]
fn test_fennec_with_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    std::fs::write(&config_path, r#"
[provider]
default_model = "gpt-4"
timeout_seconds = 30

[security]
default_sandbox_level = "read-only"
audit_log_enabled = false

[memory]
max_transcript_size = 5000
enable_agents_md = true

[tui]
theme = "default"

[tui.key_bindings]
quit = "Ctrl+C"
help = "F1"
clear = "Ctrl+L"
"#).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--bin", "fennec", "--", "--config", config_path.to_str().unwrap(), "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
}