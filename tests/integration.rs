//! Integration tests for tracey

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::thread;

/// Find an available port
fn get_available_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// Create a test project with rule references
fn create_test_project(dir: &PathBuf) {
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join(".config/tracey")).unwrap();

    // Rust file with rule references
    std::fs::write(
        dir.join("src/lib.rs"),
        r#"
//! Test library

/// Implements [test.rule.one]
pub fn foo() {}

// See [test.rule.two] for details
pub fn bar() {}

/* Reference to [test.rule.three] in block comment */
pub fn baz() {}

// Invalid reference: [nonexistent.rule]
pub fn qux() {}
"#,
    )
    .unwrap();

    // Cargo.toml so tracey finds the project root
    std::fs::write(
        dir.join("Cargo.toml"),
        r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();
}

/// Get the JSON response for the mock server
fn get_rules_json() -> String {
    r##"{
  "rules": {
    "test.rule.one": {"url": "#r-test.rule.one"},
    "test.rule.two": {"url": "#r-test.rule.two"},
    "test.rule.three": {"url": "#r-test.rule.three"},
    "test.rule.four": {"url": "#r-test.rule.four"}
  }
}"##
    .to_string()
}

/// Start a simple HTTP server that serves the rules.json
fn start_mock_server(port: u16) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();

        // Handle requests for the test
        for stream in listener.incoming().take(2) {
            let mut stream = stream.unwrap();

            // Read the HTTP request (consume headers until empty line)
            let buf_reader = BufReader::new(&stream);
            for line in buf_reader.lines() {
                let line = line.unwrap();
                if line.is_empty() {
                    break;
                }
            }

            // Send response
            let body = get_rules_json();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
            stream.flush().unwrap();
        }
    })
}

#[test]
fn test_coverage_report() {
    // Build tracey first
    let status = Command::new("cargo")
        .args(["build"])
        .status()
        .expect("Failed to build tracey");
    assert!(status.success(), "cargo build failed");

    // Create temp directory for test project
    let temp_dir = std::env::temp_dir().join(format!("tracey-test-{}", std::process::id()));
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    create_test_project(&temp_dir);

    // Start mock server
    let port = get_available_port();
    let _server = start_mock_server(port);

    // Give server time to start
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Write config - using facet-kdl format with child nodes
    std::fs::write(
        temp_dir.join(".config/tracey/config.kdl"),
        format!(
            r#"spec {{
    name "test-spec"
    rules_url "http://127.0.0.1:{}/rules.json"
}}"#,
            port
        ),
    )
    .unwrap();

    // Run tracey
    let tracey_bin = std::env::current_dir().unwrap().join("target/debug/tracey");

    let output = Command::new(&tracey_bin)
        .current_dir(&temp_dir)
        .args(["-v"])
        .output()
        .expect("Failed to run tracey");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("stdout:\n{}", stdout);
    println!("stderr:\n{}", stderr);

    // Verify coverage: 3 rules covered out of 4
    assert!(
        stdout.contains("3/4 rules"),
        "Expected 3/4 rules in output, got:\n{}",
        stdout
    );

    // Verify uncovered rule is reported
    assert!(
        stdout.contains("test.rule.four"),
        "Expected test.rule.four in uncovered rules"
    );

    // Verify invalid reference is reported
    assert!(
        stdout.contains("nonexistent.rule"),
        "Expected nonexistent.rule in invalid references"
    );

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_check_mode_fails_on_threshold() {
    // Build tracey first
    let status = Command::new("cargo")
        .args(["build"])
        .status()
        .expect("Failed to build tracey");
    assert!(status.success(), "cargo build failed");

    // Create temp directory for test project
    let temp_dir = std::env::temp_dir().join(format!("tracey-test-check-{}", std::process::id()));
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    create_test_project(&temp_dir);

    // Start mock server
    let port = get_available_port();
    let _server = start_mock_server(port);
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Write config - using facet-kdl format with child nodes
    std::fs::write(
        temp_dir.join(".config/tracey/config.kdl"),
        format!(
            r#"spec {{
    name "test-spec"
    rules_url "http://127.0.0.1:{}/rules.json"
}}"#,
            port
        ),
    )
    .unwrap();

    // Run tracey with --check and a high threshold (should fail)
    let tracey_bin = std::env::current_dir().unwrap().join("target/debug/tracey");

    let output = Command::new(&tracey_bin)
        .current_dir(&temp_dir)
        .args(["--check", "--threshold", "100"])
        .output()
        .expect("Failed to run tracey");

    // Should exit with code 1 because we only have 75% coverage
    assert!(
        !output.status.success(),
        "Expected tracey to fail with 100% threshold"
    );

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).ok();
}
