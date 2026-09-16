use std::fmt::Write;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, Instant};

use fe2o3_kfd::engineering_wire::{write_header_v1, CommandV1, ResponseV1};

use crate::session::{Session, Transport};

#[test]
fn blocked_worker_pipe_is_bounded_and_only_own_child_is_reaped() {
    // This child is a local shell protocol stub. It never opens any GPU device.
    let directory =
        std::env::temp_dir().join(format!("ferric-finite-pipe-test-{}", std::process::id()));
    fs::create_dir(&directory).unwrap();
    let script = directory.join("worker");
    let mut frame = vec![];
    write_header_v1(
        &mut frame,
        &ResponseV1::Ready {
            protocol: 1,
            target: "gfx950:xnack-".into(),
            device_unique_id: 7,
            authority: "none".into(),
        },
    )
    .unwrap();
    let mut encoded = String::new();
    for byte in frame {
        write!(&mut encoded, "\\{byte:03o}").unwrap();
    }
    fs::write(
        &script,
        format!("#!/bin/sh\nprintf '{encoded}'\nwhile :; do :; done\n"),
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    let stderr = crate::create_private(&directory.join("stderr")).unwrap();
    let started = Instant::now();
    let (mut session, ready) =
        Session::spawn_with_deadline(&script, 7, stderr, Duration::from_millis(100)).unwrap();
    crate::probe::ready(&ready, 7).unwrap();
    let pid = session.process_id();
    assert!(session
        .request(
            CommandV1::Write {
                buffer: 1,
                offset: 0,
                payload_bytes: 1024 * 1024
            },
            vec![0; 1024 * 1024]
        )
        .is_err());
    drop(session);
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn private_selector_rejects_public_file_zero_and_symlink() {
    let directory = std::env::temp_dir().join(format!(
        "ferric-finite-selector-test-{}",
        std::process::id()
    ));
    fs::create_dir(&directory).unwrap();
    let path = directory.join("selector");
    crate::write_private(&path, b"7\n").unwrap();
    assert_eq!(crate::private_device_id(&path).unwrap(), 7);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(crate::private_device_id(&path).is_err());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let link = directory.join("link");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    assert!(crate::private_device_id(&link).is_err());
    fs::write(&path, b"0").unwrap();
    assert!(crate::private_device_id(&path).is_err());
    fs::remove_dir_all(directory).unwrap();
}
