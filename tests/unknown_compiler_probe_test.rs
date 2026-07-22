#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::time::Instant;

fn kache_binary() -> &'static str {
    env!("CARGO_BIN_EXE_kache")
}

#[test]
fn probe_recovers_when_wrapper_fork_bombs() {
    let dir = tempfile::tempdir().unwrap();
    let wrapper = dir.path().join("my-compiler");
    
    fs::write(
        &wrapper,
        format!("#!/bin/sh\nexec {} \"$0\" \"$@\"\n", kache_binary()),
    )
    .unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();

    let start = Instant::now();
    let _ = Command::new(kache_binary())
        .arg(&wrapper)
        .arg("-c")
        .arg("foo.c")
        .env("KACHE_CACHE_DIR", dir.path().join("cache"))
        .output()
        .expect("failed to run kache");

    assert!(start.elapsed().as_secs() < 10, "probe should not hang on fork bomb");
}

#[test]
fn probe_recovers_when_wrapper_emits_8kb_then_hangs() {
    let dir = tempfile::tempdir().unwrap();
    let wrapper = dir.path().join("my-compiler-hang");
    
    fs::write(
        &wrapper,
        "#!/bin/sh\n\
        printf 'A%.0s' {1..9000}\n\
        sleep 60\n",
    )
    .unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();

    let start = Instant::now();
    let _ = Command::new(kache_binary())
        .arg(&wrapper)
        .arg("-c")
        .arg("foo.c")
        .env("KACHE_CACHE_DIR", dir.path().join("cache"))
        .output()
        .expect("failed to run kache");

    assert!(start.elapsed().as_secs() < 15, "probe must kill hanging wrapper after reading 8KB");
}

#[test]
fn probe_recovers_when_wrapper_leaves_descendant_on_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let wrapper = dir.path().join("my-compiler-descendant");
    
    fs::write(
        &wrapper,
        "#!/bin/sh\n\
        (sleep 60) &\n\
        exit 0\n",
    )
    .unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();

    let start = Instant::now();
    let _ = Command::new(kache_binary())
        .arg(&wrapper)
        .arg("-c")
        .arg("foo.c")
        .env("KACHE_CACHE_DIR", dir.path().join("cache"))
        .output()
        .expect("failed to run kache");

    assert!(start.elapsed().as_secs() < 15, "probe must kill descendants holding stdout");
}
