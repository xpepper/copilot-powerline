//! Runs the built binary against a throwaway `HOME` to check what `--init`
//! writes and how it fails. `main` owns this flow, so unit tests cannot see it.
#![cfg(unix)]

mod common;

use common::{Sandbox, assert_shows};
use std::fs;

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn test_init_writes_a_config_the_binary_reads_back() {
    let sandbox = Sandbox::new();
    assert_shows(&sandbox.refresh_plain(), "1%");

    let output = sandbox.run(&["--init"]);
    assert!(output.status.success(), "{}", stderr(&output));

    // An unparsable file is ignored silently, so prove the file is read by
    // changing a value and watching the status line follow.
    let path = sandbox.copilot_dir().join("powerline.toml");
    let written = fs::read_to_string(&path).unwrap();
    assert!(written.contains("show_percentage = true"));
    fs::write(
        &path,
        written.replace("show_percentage = true", "show_percentage = false"),
    )
    .unwrap();
    let line = sandbox.refresh_plain();
    assert!(!line.contains("1%"), "edited value ignored: {line}");
}

#[test]
fn test_init_leaves_the_status_line_unchanged() {
    let sandbox = Sandbox::new();
    let before = sandbox.refresh_plain();

    assert!(sandbox.run(&["--init"]).status.success());

    assert_eq!(sandbox.refresh_plain(), before);
}

#[test]
fn test_init_never_overwrites_an_existing_config() {
    let sandbox = Sandbox::new();
    sandbox.write_config("style = \"capsule\"\n");

    let output = sandbox.run(&["--init"]);

    assert!(output.status.success());
    assert!(stderr(&output).contains("already exists"));
    let kept = fs::read_to_string(sandbox.copilot_dir().join("powerline.toml")).unwrap();
    assert_eq!(kept, "style = \"capsule\"\n");
}

#[test]
fn test_init_writes_to_the_path_given_with_config() {
    let sandbox = Sandbox::new();
    let target = sandbox.home().join("elsewhere").join("custom.toml");

    let output = sandbox.run(&["--init", "--config", target.to_str().unwrap()]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(fs::read_to_string(&target).unwrap().contains("[tokens]"));
    assert!(!sandbox.copilot_dir().join("powerline.toml").exists());
}

#[test]
fn test_init_fails_when_the_config_cannot_be_written() {
    let sandbox = Sandbox::new();
    // A file where the `.copilot` directory should be.
    fs::write(sandbox.copilot_dir(), "not a directory").unwrap();

    let output = sandbox.run(&["--init"]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains(".copilot"), "{}", stderr(&output));
}

#[test]
fn test_init_does_not_write_through_a_dangling_symlink() {
    let sandbox = Sandbox::new();
    let link = sandbox.home().join("link.toml");
    let destination = sandbox.home().join("destination.toml");
    std::os::unix::fs::symlink(&destination, &link).unwrap();

    let output = sandbox.run(&["--init", "--config", link.to_str().unwrap()]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stderr(&output).contains("already exists"));
    assert!(!destination.exists(), "wrote through the symlink");
}
