//! The production command runner, run for real: where a command runs, what it
//! sees, what it may print, and that nothing it starts outlives it.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::os::unix::process::ExitStatusExt as _;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use std::time::Instant;

use config::catalogue::BASE_COMMAND_ENVIRONMENT;
use config::consent::{
    CommandFailure, CommandPolicy, CommandRunner as _, Environment,
    FailureCause, RepositoryRoots, StartFailure,
};
use config_adapters::credentials::BashCommandRunner;
use rustix::process::Pid;
use rustix::process::Signal;

const HARNESS: &str = "ACCELERATOR_RUNNER_HARNESS";

struct Map(BTreeMap<String, String>);

impl Environment for Map {
    fn read(&self, name: &str) -> Option<String> {
        self.0.get(name).cloned()
    }
}

/// A repository, a directory outside it, and a base for the runner's
/// temporary working directories, each its own temporary directory.
struct Fixture {
    _dirs: Vec<tempfile::TempDir>,
    repo: PathBuf,
    outside: PathBuf,
    temp_base: PathBuf,
}

fn canonical_tempdir(tag: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::Builder::new()
        .prefix(&format!("runner-{tag}-"))
        .tempdir()
        .unwrap();
    let path = dir.path().canonicalize().unwrap();
    (dir, path)
}

impl Fixture {
    fn new() -> Self {
        let (repo_dir, repo) = canonical_tempdir("repo");
        let (outside_dir, outside) = canonical_tempdir("outside");
        let (base_dir, temp_base) = canonical_tempdir("base");
        Self {
            _dirs: vec![repo_dir, outside_dir, base_dir],
            repo,
            outside,
            temp_base,
        }
    }

    fn parent(&self) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("PATH".to_owned(), system_path()),
            ("HOME".to_owned(), self.outside.display().to_string()),
        ])
    }

    fn runner(&self) -> BashCommandRunner {
        self.runner_with(self.parent())
    }

    fn runner_with(
        &self,
        parent: BTreeMap<String, String>,
    ) -> BashCommandRunner {
        BashCommandRunner::new(
            RepositoryRoots::complete(vec![self.repo.clone()]),
            Box::new(Map(parent)),
            self.temp_base.clone(),
        )
    }

    fn pidfile(&self, name: &str) -> PathBuf {
        self.outside.join(name)
    }

    fn leftover_working_directories(&self) -> Vec<PathBuf> {
        leftovers(&self.temp_base)
    }
}

fn leftovers(base: &Path) -> Vec<PathBuf> {
    fs::read_dir(base)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("accelerator-cmd-"))
        })
        .collect()
}

fn system_path() -> String {
    std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".to_owned())
}

fn policy(timeout: Duration) -> CommandPolicy {
    CommandPolicy::for_test(timeout, BASE_COMMAND_ENVIRONMENT)
}

fn base() -> CommandPolicy {
    policy(Duration::from_secs(10))
}

fn github() -> CommandPolicy {
    let mut admitted = BASE_COMMAND_ENVIRONMENT.to_vec();
    admitted.extend(["GH_HOST", "GH_CONFIG_DIR"]);
    CommandPolicy::for_test(Duration::from_secs(10), &admitted)
}

fn recorded_pid(pidfile: &Path) -> i32 {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Ok(text) = fs::read_to_string(pidfile) {
            if let Ok(pid) = text.trim().parse() {
                return pid;
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("no pid was recorded at {}", pidfile.display());
}

fn is_gone(pid: i32) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        let stat = Command::new("ps")
            .args(["-o", "stat=", "-p", &pid.to_string()])
            .output()
            .unwrap();
        let state = String::from_utf8_lossy(&stat.stdout);
        if state.trim().is_empty() || state.trim_start().starts_with('Z') {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

fn kill(pid: i32) {
    if let Some(pid) = Pid::from_raw(pid) {
        let _ = rustix::process::kill_process(pid, Signal::KILL);
    }
}

fn background_child(pidfile: &Path) -> String {
    format!("sleep 60 & echo $! > {}; ", pidfile.display())
}

fn over_the_cap() -> String {
    format!("head -c {} /dev/zero | tr '\\0' a; ", 65_537)
}

#[test]
fn each_run_gets_a_fresh_working_directory_that_is_removed_afterwards() {
    let fixture = Fixture::new();
    let runner = fixture.runner();

    let first = runner.run("pwd", &base()).unwrap();
    let second = runner.run("pwd", &base()).unwrap();

    assert_ne!(first, second);
    assert!(!Path::new(&first).exists());
    assert!(!Path::new(&second).exists());

    let short = policy(Duration::from_secs(1));
    assert_eq!(
        runner.run("sleep 30", &short),
        Err(CommandFailure::TimedOut)
    );
    assert_eq!(
        runner.run(&over_the_cap(), &base()),
        Err(CommandFailure::OutputExceeded)
    );
    assert_eq!(
        fixture.leftover_working_directories(),
        Vec::<PathBuf>::new()
    );
}

#[test]
fn the_working_directory_is_never_home_or_inside_the_repository() {
    let fixture = Fixture::new();
    let mut parent = fixture.parent();
    parent.insert("HOME".to_owned(), fixture.repo.display().to_string());
    let runner = fixture.runner_with(parent.clone());

    let cwd = PathBuf::from(runner.run("pwd -P", &base()).unwrap());

    assert!(!cwd.starts_with(&fixture.repo), "{}", cwd.display());

    let inside_base = fixture.repo.join("tmp");
    fs::create_dir_all(&inside_base).unwrap();
    let runner = BashCommandRunner::new(
        RepositoryRoots::complete(vec![fixture.repo.clone()]),
        Box::new(Map(parent)),
        inside_base.clone(),
    );

    let cwd = PathBuf::from(runner.run("pwd -P", &base()).unwrap());

    assert!(!cwd.starts_with(&fixture.repo), "{}", cwd.display());
    assert!(!cwd.starts_with(&inside_base), "{}", cwd.display());
}

#[test]
fn a_repository_enclosing_every_temporary_directory_cannot_start() {
    let fixture = Fixture::new();
    let runner = BashCommandRunner::new(
        RepositoryRoots::complete(vec![PathBuf::from("/")]),
        Box::new(Map(fixture.parent())),
        fixture.temp_base,
    );

    let outcome = runner.run("true", &base());

    assert_eq!(
        outcome,
        Err(CommandFailure::Failed(FailureCause::CouldNotStart(
            StartFailure::NoWorkingDirectoryOutsideTheRepository
        )))
    );
}

#[test]
fn a_waited_on_background_child_times_out_and_is_killed() {
    let fixture = Fixture::new();
    let pidfile = fixture.pidfile("child");
    let started = Instant::now();

    let outcome = fixture.runner().run(
        &format!("{}wait", background_child(&pidfile)),
        &policy(Duration::from_secs(1)),
    );

    assert_eq!(outcome, Err(CommandFailure::TimedOut));
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(is_gone(recorded_pid(&pidfile)));
}

#[test]
fn a_background_child_left_behind_is_killed_once_the_leader_exits() {
    let fixture = Fixture::new();
    let pidfile = fixture.pidfile("child");
    let started = Instant::now();

    let outcome = fixture.runner().run(
        &format!("{}echo ok", background_child(&pidfile)),
        &policy(Duration::from_secs(30)),
    );

    assert_eq!(outcome, Ok("ok".to_owned()));
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(is_gone(recorded_pid(&pidfile)));
}

#[test]
fn a_detached_descendant_costs_at_most_the_grace_period() {
    let fixture = Fixture::new();
    let pidfile = fixture.pidfile("detached");
    let started = Instant::now();

    let outcome = fixture.runner().run(
        &format!(
            "perl -e 'use POSIX; setsid; open(my $f, \">\", \"{pid}\"); \
             print $f \"$$\\n\"; close($f); exec @ARGV' sleep 60 & \
             while [ ! -s {pid} ]; do :; done; echo ok",
            pid = pidfile.display()
        ),
        &policy(Duration::from_secs(30)),
    );

    let detached = recorded_pid(&pidfile);
    kill(detached);
    assert_eq!(outcome, Ok("ok".to_owned()));
    assert!(started.elapsed() < Duration::from_secs(5));
}

fn fake(dir: &Path, name: &str, marker: &Path) {
    fs::create_dir_all(dir).unwrap();
    let path = dir.join(name);
    fs::write(
        &path,
        format!("#!/bin/sh\ntouch {}\nexit 0\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn path_entries(printed: &str) -> Vec<String> {
    printed.split(':').map(str::to_owned).collect()
}

#[test]
fn relative_empty_and_in_repository_path_entries_are_dropped() {
    let fixture = Fixture::new();
    let repo_bin = fixture.repo.join("bin");
    let marker = fixture.outside.join("fake-ran");
    fake(&repo_bin, "bash", &marker);
    fake(&repo_bin, "gh", &marker);
    let mut parent = fixture.parent();
    parent.insert(
        "PATH".to_owned(),
        format!(
            ":bin:{}:{}:{}",
            fixture.repo.join("not-yet").display(),
            repo_bin.display(),
            system_path()
        ),
    );

    let printed = fixture
        .runner_with(parent)
        .run("gh >/dev/null 2>&1; printf %s \"$PATH\"", &base())
        .unwrap();

    assert!(!marker.exists(), "a repository binary ran");
    for entry in path_entries(&printed) {
        assert!(!entry.is_empty(), "{printed}");
        assert!(Path::new(&entry).is_absolute(), "{printed}");
        assert!(
            !entry.starts_with(&*fixture.repo.to_string_lossy()),
            "{printed}"
        );
    }
}

#[test]
fn a_path_holding_no_bash_cannot_start() {
    let fixture = Fixture::new();
    let mut parent = fixture.parent();
    parent.insert("PATH".to_owned(), fixture.outside.display().to_string());

    let outcome = fixture.runner_with(parent).run("true", &base());

    assert_eq!(
        outcome,
        Err(CommandFailure::Failed(FailureCause::CouldNotStart(
            StartFailure::NoBashOnPath
        )))
    );
}

#[test]
fn locator_variables_inside_the_repository_are_dropped() {
    let fixture = Fixture::new();
    for variable in ["XDG_CONFIG_HOME", "XDG_RUNTIME_DIR", "GH_CONFIG_DIR"] {
        let mut inside = fixture.parent();
        inside.insert(variable.to_owned(), fixture.repo.display().to_string());
        let mut outside = fixture.parent();
        outside
            .insert(variable.to_owned(), fixture.outside.display().to_string());
        let probe = format!("printf %s \"${{{variable}-unset}}\"");

        assert_eq!(
            fixture.runner_with(inside).run(&probe, &github()).unwrap(),
            "unset",
            "{variable}"
        );
        assert_eq!(
            fixture.runner_with(outside).run(&probe, &github()).unwrap(),
            fixture.outside.display().to_string(),
            "{variable}"
        );
    }
}

fn filler(bytes: usize, stream: &str) -> String {
    let redirect = if stream == "stderr" { " >&2" } else { "" };
    format!("head -c {bytes} /dev/zero | tr '\\0' a{redirect}; ")
}

#[test]
fn output_is_capped_across_stdout_and_stderr_combined() {
    let fixture = Fixture::new();
    let runner = fixture.runner();

    let exact = runner.run(&filler(65_536, "stdout"), &base()).unwrap();
    assert_eq!(exact.len(), 65_536);

    for command in [
        filler(65_537, "stdout"),
        filler(65_537, "stderr"),
        format!("{}{}", filler(32_768, "stdout"), filler(32_769, "stderr")),
        filler(1 << 20, "stdout"),
    ] {
        assert_eq!(
            runner.run(&command, &base()),
            Err(CommandFailure::OutputExceeded),
            "{command}"
        );
    }
}

#[test]
fn a_helper_that_passes_the_cap_is_killed_early_with_its_children() {
    let fixture = Fixture::new();
    let pidfile = fixture.pidfile("child");
    let thirty = policy(Duration::from_secs(30));

    let started = Instant::now();
    let outcome = fixture.runner().run(
        &format!("{}{}sleep 60", background_child(&pidfile), over_the_cap()),
        &thirty,
    );

    assert_eq!(outcome, Err(CommandFailure::OutputExceeded));
    assert!(started.elapsed() < Duration::from_secs(10));
    assert!(is_gone(recorded_pid(&pidfile)));

    let started = Instant::now();
    let outcome = fixture
        .runner()
        .run(&format!("{}sleep 60", over_the_cap()), &thirty);

    assert_eq!(outcome, Err(CommandFailure::OutputExceeded));
    assert!(started.elapsed() < Duration::from_secs(10));
}

fn environment_names(printed: &str) -> Vec<String> {
    let mut names: Vec<String> = printed
        .lines()
        .filter_map(|line| {
            line.split_once('=').map(|(name, _)| name.to_owned())
        })
        .filter(|name| !matches!(name.as_str(), "PWD" | "SHLVL" | "_"))
        .collect();
    names.sort();
    names
}

#[test]
fn a_command_sees_only_its_admitted_environment() {
    let fixture = Fixture::new();
    let xdg_config = fixture.outside.join("config");
    let xdg_runtime = fixture.outside.join("runtime");
    let gh_config = fixture.outside.join("gh");
    for dir in [&xdg_config, &xdg_runtime, &gh_config] {
        fs::create_dir_all(dir).unwrap();
    }
    let mut parent = fixture.parent();
    for (name, value) in [
        ("TERM", "xterm".to_owned()),
        ("XDG_CONFIG_HOME", xdg_config.display().to_string()),
        ("XDG_RUNTIME_DIR", xdg_runtime.display().to_string()),
        ("DBUS_SESSION_BUS_ADDRESS", "unix:path=/run/bus".to_owned()),
        ("GH_HOST", "github.example.com".to_owned()),
        ("GH_CONFIG_DIR", gh_config.display().to_string()),
        ("GH_TOKEN", "leaked".to_owned()),
        ("AWS_SECRET_ACCESS_KEY", "leaked".to_owned()),
        ("UNRELATED", "leaked".to_owned()),
    ] {
        parent.insert(name.to_owned(), value);
    }
    let runner = fixture.runner_with(parent.clone());

    let mut expected: Vec<String> = BASE_COMMAND_ENVIRONMENT
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    expected.sort();
    assert_eq!(
        environment_names(&runner.run("env", &base()).unwrap()),
        expected
    );

    let printed = runner.run("env", &github()).unwrap();
    expected.extend(["GH_CONFIG_DIR".to_owned(), "GH_HOST".to_owned()]);
    expected.sort();
    assert_eq!(environment_names(&printed), expected);
    assert!(printed.contains("GH_HOST=github.example.com"), "{printed}");

    parent.remove("TERM");
    let printed = fixture.runner_with(parent).run("env", &base()).unwrap();
    assert!(!environment_names(&printed).contains(&"TERM".to_owned()));
}

#[test]
fn stdin_is_at_end_of_file() {
    let fixture = Fixture::new();
    let started = Instant::now();

    let printed = fixture.runner().run("cat", &base()).unwrap();

    assert_eq!(printed, "");
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn surrounding_whitespace_is_trimmed() {
    let fixture = Fixture::new();
    let runner = fixture.runner();

    assert_eq!(runner.run("printf 'tok\\r\\n'", &base()).unwrap(), "tok");
    assert_eq!(runner.run("printf '  tok  '", &base()).unwrap(), "tok");
}

#[test]
fn a_helper_that_prompts_on_the_terminal_fails_rather_than_prompting() {
    let fixture = Fixture::new();
    let started = Instant::now();

    let outcome = fixture
        .runner()
        .run("read -r line < /dev/tty", &policy(Duration::from_secs(1)));

    assert!(
        matches!(
            outcome,
            Err(CommandFailure::Failed(_) | CommandFailure::TimedOut)
        ),
        "{outcome:?}"
    );
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn a_leader_exiting_just_before_the_deadline_returns_by_it() {
    let fixture = Fixture::new();
    let started = Instant::now();

    let outcome = fixture.runner().run(
        "sleep 60 & sleep 0.9; echo ok",
        &policy(Duration::from_secs(1)),
    );

    assert!(
        started.elapsed() < Duration::from_millis(1_250),
        "{:?} then {outcome:?}",
        started.elapsed()
    );
}

#[test]
fn a_caching_agent_in_the_group_gets_its_grace_period() {
    let fixture = Fixture::new();
    let written = fixture.outside.join("cache-written");

    let outcome = fixture.runner().run(
        &format!(
            "(trap 'sleep 0.2; echo done > {file}; exit 0' TERM; \
             exec >/dev/null 2>&1 </dev/null; \
             while :; do sleep 0.05; done) & sleep 0.2; echo ok",
            file = written.display()
        ),
        &policy(Duration::from_secs(30)),
    );

    assert_eq!(outcome, Ok("ok".to_owned()));
    assert!(written.exists(), "the agent was killed before it finished");
}

#[test]
fn a_helper_with_no_descendants_returns_promptly() {
    let fixture = Fixture::new();
    let runner = fixture.runner();
    runner.run("true", &base()).unwrap();
    let started = Instant::now();

    let printed = runner.run("printf tok", &base()).unwrap();

    assert_eq!(printed, "tok");
    assert!(
        started.elapsed() < Duration::from_millis(300),
        "{:?}",
        started.elapsed()
    );
}

/// The re-executed side of the interrupt tests: a no-op unless the parent
/// set the harness scenario.
#[test]
fn interrupt_harness() {
    let Ok(scenario) = std::env::var(HARNESS) else {
        return;
    };
    let temp_base = PathBuf::from(std::env::var("HARNESS_TEMP_BASE").unwrap());
    let pidfile = std::env::var("HARNESS_PIDFILE").unwrap();
    let ready = PathBuf::from(std::env::var("HARNESS_READY").unwrap());
    let signal: i32 = std::env::var("HARNESS_SIGNAL").unwrap().parse().unwrap();
    let outside = temp_base.parent().unwrap().to_path_buf();
    let parent = BTreeMap::from([
        ("PATH".to_owned(), system_path()),
        ("HOME".to_owned(), outside.display().to_string()),
    ]);
    let runner = BashCommandRunner::new(
        RepositoryRoots::complete(Vec::new()),
        Box::new(Map(parent)),
        temp_base,
    );
    let long_helper = format!("echo $$ > {pidfile}; exec sleep 60");
    let announce = || {
        fs::write(&ready, "").unwrap();
        std::thread::sleep(Duration::from_secs(30));
    };
    match scenario.as_str() {
        "during" => {
            let _ = runner.run(&long_helper, &policy(Duration::from_secs(30)));
        }
        "after-return" => {
            runner.run("true", &base()).unwrap();
            announce();
        }
        "ignored" => {
            // SAFETY: setting a disposition before any thread observes it.
            unsafe { libc::signal(signal, libc::SIG_IGN) };
            let _ = runner.run(
                &format!("echo $$ > {pidfile}; sleep 1"),
                &policy(Duration::from_secs(30)),
            );
            fs::write(&ready, "").unwrap();
            std::thread::sleep(Duration::from_millis(500));
            std::process::exit(0);
        }
        "after-spawn-failure" => {
            let nowhere = BashCommandRunner::new(
                RepositoryRoots::complete(Vec::new()),
                Box::new(Map(BTreeMap::from([(
                    "PATH".to_owned(),
                    outside.display().to_string(),
                )]))),
                outside.clone(),
            );
            let _ = nowhere.run("true", &base());
            announce();
        }
        "after-timeout" => {
            let _ = runner.run("sleep 30", &policy(Duration::from_secs(1)));
            announce();
        }
        "after-output-exceeded" => {
            let _ = runner.run(&over_the_cap(), &base());
            announce();
        }
        other => panic!("unknown scenario {other}"),
    }
}

struct Harness {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Harness {
    fn new() -> Self {
        let (dir, root) = canonical_tempdir("harness");
        fs::create_dir_all(root.join("base")).unwrap();
        Self { _dir: dir, root }
    }

    fn pidfile(&self) -> PathBuf {
        self.root.join("helper.pid")
    }

    fn ready(&self) -> PathBuf {
        self.root.join("ready")
    }

    fn spawn(&self, scenario: &str, signal: Signal) -> std::process::Child {
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "interrupt_harness", "--nocapture"])
            .env(HARNESS, scenario)
            .env("HARNESS_TEMP_BASE", self.root.join("base"))
            .env("HARNESS_PIDFILE", self.pidfile())
            .env("HARNESS_READY", self.ready())
            .env("HARNESS_SIGNAL", signal.as_raw().to_string())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap()
    }

    fn await_file(path: &Path) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !path.exists() {
            assert!(
                Instant::now() < deadline,
                "{} never appeared",
                path.display()
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

fn send(child: &std::process::Child, signal: Signal) {
    let pid = Pid::from_child(child);
    rustix::process::kill_process(pid, signal).unwrap();
}

const FORWARDED: [Signal; 3] = [Signal::INT, Signal::TERM, Signal::HUP];

#[test]
fn an_interrupt_during_a_run_tears_the_helper_down_then_ends_the_cli() {
    for signal in FORWARDED {
        let harness = Harness::new();
        let mut child = harness.spawn("during", signal);
        let helper = recorded_pid(&harness.pidfile());
        std::thread::sleep(Duration::from_millis(50));

        let sent = Instant::now();
        send(&child, signal);
        let status = child.wait().unwrap();

        assert_eq!(status.signal(), Some(signal.as_raw()), "{signal:?}");
        assert!(
            sent.elapsed() < Duration::from_millis(300),
            "{signal:?} took {:?}",
            sent.elapsed()
        );
        assert!(is_gone(helper), "{signal:?}");
        assert_eq!(
            leftovers(&harness.root.join("base")),
            Vec::<PathBuf>::new()
        );
    }
}

#[test]
fn an_interrupt_after_a_run_ends_the_cli_by_default() {
    for scenario in [
        "after-return",
        "after-spawn-failure",
        "after-timeout",
        "after-output-exceeded",
    ] {
        for signal in FORWARDED {
            let harness = Harness::new();
            let mut child = harness.spawn(scenario, signal);
            Harness::await_file(&harness.ready());

            send(&child, signal);
            let status = child.wait().unwrap();

            assert_eq!(
                status.signal(),
                Some(signal.as_raw()),
                "{scenario} {signal:?}"
            );
        }
    }
}

#[test]
fn an_ignored_signal_stays_ignored_during_and_after_a_run() {
    for signal in FORWARDED {
        let harness = Harness::new();
        let mut child = harness.spawn("ignored", signal);
        recorded_pid(&harness.pidfile());

        send(&child, signal);
        Harness::await_file(&harness.ready());
        send(&child, signal);
        let status = child.wait().unwrap();

        assert_eq!(status.code(), Some(0), "{signal:?}: {status:?}");
    }
}
