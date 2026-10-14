#![cfg(unix)]
#![allow(clippy::expect_used)]

use std::fs::File;
use std::os::fd::AsRawFd as _;
use std::sync::Mutex;
use std::sync::MutexGuard;

use process::descriptors_above_stdio;

/// The tests share one descriptor table, so a number one test closes could be
/// reused by another's open before the listing runs.
static DESCRIPTOR_TABLE: Mutex<()> = Mutex::new(());

fn exclusive_table() -> MutexGuard<'static, ()> {
    DESCRIPTOR_TABLE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn listed() -> Vec<i32> {
    descriptors_above_stdio().expect("list the open descriptors")
}

#[test]
fn an_open_descriptor_is_listed() {
    let _table = exclusive_table();
    let file = File::open("/dev/null").expect("open");
    assert!(listed().contains(&file.as_raw_fd()));
}

#[test]
fn a_closed_descriptor_is_not_listed() {
    let _table = exclusive_table();
    let file = File::open("/dev/null").expect("open");
    let descriptor = file.as_raw_fd();
    drop(file);
    assert!(!listed().contains(&descriptor));
}

#[test]
fn stdio_is_never_listed() {
    let _table = exclusive_table();
    assert!(listed().iter().all(|descriptor| *descriptor > 2));
}
