use std::fs;
use std::process::Command;
use std::path::Path;

fn strip_ansi(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            i += 2;
            while i < bytes.len() && bytes[i] != b'm' {
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

fn run_tachyon(dir: &Path, args: &[&str]) -> (bool, String, String) {
    let tachyon_bin = env!("CARGO_BIN_EXE_tachyon");
    let output = Command::new(tachyon_bin)
        .current_dir(dir)
        .args(args)
        .output()
        .expect("Failed to execute tachyon");

    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    (output.status.success(), stdout, stderr)
}

fn run_git(dir: &Path, args: &[&str]) -> (bool, String, String) {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .expect("Failed to execute git");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.success(), stdout, stderr)
}

#[test]
fn test_cli_help() {
    let temp_dir = std::env::temp_dir();
    let (ok, stdout, _) = run_tachyon(&temp_dir, &["--help"]);
    assert!(ok);
    assert!(stdout.contains("Tachyon"));
    assert!(stdout.contains("stage"));
    assert!(stdout.contains("save"));
    assert!(stdout.contains("jump"));
    assert!(stdout.contains("history"));
    assert!(stdout.contains("shelf"));
}

#[test]
fn test_full_repository_lifecycle_and_git_interop() {
    let test_dir = std::env::temp_dir().join(format!("tachyon_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    // 1. tachyon init
    let (ok, stdout, _) = run_tachyon(&test_dir, &["init"]);
    assert!(ok, "Init failed");
    assert!(stdout.contains("Initialized empty Tachyon/Git repository"));
    assert!(test_dir.join(".git").exists());

    // 2. Create files
    fs::write(test_dir.join("hello.txt"), "Hello, Tachyon!\n").unwrap();
    fs::write(test_dir.join("code.rs"), "fn speed() { 17; }\n").unwrap();

    // 3. Status untracked
    let (ok, stdout, _) = run_tachyon(&test_dir, &["status"]);
    assert!(ok);
    assert!(stdout.contains("hello.txt"));
    assert!(stdout.contains("code.rs"));

    // 4. Stage files
    let (ok, stdout, stderr) = run_tachyon(&test_dir, &["stage", "-a"]);
    println!("DEBUG STAGE STDOUT: '{}' STDERR: '{}'", stdout, stderr);
    assert!(ok);
    assert!(stdout.contains("Staged 2 file(s)"));

    // 5. Commit with save
    let (ok, stdout, _) = run_tachyon(&test_dir, &["save", "-m", "feat: first commit"]);
    assert!(ok);
    assert!(stdout.contains("feat: first commit"));

    // 6. Verify with official Git CLI
    let (git_ok, git_stdout, _) = run_git(&test_dir, &["status", "--porcelain"]);
    assert!(git_ok);
    assert_eq!(git_stdout.trim(), ""); // Clean tree

    let (git_ok, git_log, _) = run_git(&test_dir, &["log", "-n", "1", "--oneline"]);
    assert!(git_ok);
    assert!(git_log.contains("feat: first commit"));

    // 7. Branching & Switching
    let (ok, _, _) = run_tachyon(&test_dir, &["jump", "-b", "experiment"]);
    assert!(ok);

    fs::write(test_dir.join("experiment.txt"), "quantum state\n").unwrap();
    let (ok, _, _) = run_tachyon(&test_dir, &["stage", "experiment.txt"]);
    assert!(ok);

    let (ok, _, _) = run_tachyon(&test_dir, &["save", "-m", "feat: add experiment file"]);
    assert!(ok);

    // Switch back to main
    let (ok, _, _) = run_tachyon(&test_dir, &["jump", "main"]);
    assert!(ok);
    assert!(!test_dir.join("experiment.txt").exists()); // Should be gone on main

    // Combine (merge) experiment into main
    let (ok, stdout, _) = run_tachyon(&test_dir, &["combine", "experiment"]);
    assert!(ok);
    assert!(stdout.contains("Fast-forward merge"));
    assert!(test_dir.join("experiment.txt").exists()); // Now present on main

    // 8. Shelf (Stash) operations
    fs::write(test_dir.join("hello.txt"), "Hello, modified world!\n").unwrap();
    let (ok, stdout, _) = run_tachyon(&test_dir, &["shelf", "save", "-m", "wip changes"]);
    assert!(ok);
    assert!(stdout.contains("Saved working directory"));

    let (ok, stdout, _) = run_tachyon(&test_dir, &["shelf", "list"]);
    assert!(ok);
    assert!(stdout.contains("wip changes"));

    let (ok, stdout, _) = run_tachyon(&test_dir, &["shelf", "pop"]);
    assert!(ok);
    assert!(stdout.contains("Popped shelf"));

    // 9. Authors (Blame)
    let (ok, stdout, _) = run_tachyon(&test_dir, &["authors", "code.rs"]);
    assert!(ok);
    assert!(stdout.contains("Authors for: code.rs"));
    assert!(stdout.contains("fn speed() { 17; }"));

    // 10. Milestone (Tag)
    let (ok, stdout, _) = run_tachyon(&test_dir, &["milestone", "v1.0.0"]);
    assert!(ok);
    assert!(stdout.contains("Created milestone 'v1.0.0'"));

    let (ok, stdout, _) = run_tachyon(&test_dir, &["milestone"]);
    assert!(ok);
    assert!(stdout.contains("v1.0.0"));

    // 11. Run Tachyon Benchmark
    let (ok, stdout, _) = run_tachyon(&test_dir, &["bench"]);
    assert!(ok);
    assert!(stdout.contains("PERFORMANCE BENCHMARK SUITE"));
    assert!(stdout.contains("faster"));

    // Clean up
    let _ = fs::remove_dir_all(&test_dir);
}
