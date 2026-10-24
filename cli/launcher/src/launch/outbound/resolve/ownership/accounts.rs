use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The effective user's login and their primary group's name and listed
/// members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accounts {
    pub login: String,
    pub group_name: String,
    pub members: Vec<String>,
}

pub trait AccountLookup {
    fn accounts(&self, uid: u32, gid: u32) -> Option<Accounts>;
}

/// Goes through the host's NSS.
///
/// The shipped Linux launchers are static musl builds, whose own lookups read
/// only `/etc/passwd`, `/etc/group` and `nscd`, so directory-service accounts
/// (SSSD, `FreeIPA`, Active Directory, LDAP, `systemd-homed`) reach only this.
///
/// It runs `getent` only from a fixed system path, never through `PATH`,
/// since its answer decides which directories a prune trusts.
pub struct GetentLookup {
    executables: Vec<PathBuf>,
    timeout: Duration,
}

const PRODUCTION_GETENT: [&str; 2] = ["/usr/bin/getent", "/bin/getent"];
const GETENT_TIMEOUT: Duration = Duration::from_secs(2);
const ANSWER_LIMIT: usize = 64 * 1024;

impl GetentLookup {
    #[must_use]
    pub fn production() -> Self {
        Self::using(
            PRODUCTION_GETENT.iter().map(PathBuf::from).collect(),
            GETENT_TIMEOUT,
        )
    }

    #[must_use]
    pub const fn using(executables: Vec<PathBuf>, timeout: Duration) -> Self {
        Self {
            executables,
            timeout,
        }
    }

    #[must_use]
    pub fn executables(&self) -> &[PathBuf] {
        &self.executables
    }

    fn answer(&self, getent: &Path, database: &str, id: u32) -> Option<String> {
        let bytes = supervised_output(
            getent,
            &[database, &id.to_string()],
            self.timeout,
        )?;
        let answer = String::from_utf8(bytes).ok()?;
        let line = answer.strip_suffix('\n').unwrap_or(&answer);
        (!line.is_empty() && !line.contains('\n')).then(|| line.to_owned())
    }
}

impl AccountLookup for GetentLookup {
    fn accounts(&self, uid: u32, gid: u32) -> Option<Accounts> {
        let getent = self.executables.iter().find(|path| path.is_file())?;
        let login =
            parse_passwd_line(&self.answer(getent, "passwd", uid)?, uid)?;
        let (group_name, members) =
            parse_group_line(&self.answer(getent, "group", gid)?, gid)?;
        Some(Accounts {
            login,
            group_name,
            members,
        })
    }
}

#[must_use]
pub fn parse_passwd_line(line: &str, uid: u32) -> Option<String> {
    let fields: Vec<&str> = line.split(':').collect();
    let [login, _, id, _, _, _, _] = fields.as_slice() else {
        return None;
    };
    (!login.is_empty() && is_id(id, uid)).then(|| (*login).to_owned())
}

#[must_use]
pub fn parse_group_line(line: &str, gid: u32) -> Option<(String, Vec<String>)> {
    let fields: Vec<&str> = line.split(':').collect();
    let [name, _, id, members] = fields.as_slice() else {
        return None;
    };
    (!name.is_empty() && is_id(id, gid)).then(|| {
        (
            (*name).to_owned(),
            members
                .split(',')
                .filter(|member| !member.is_empty())
                .map(str::to_owned)
                .collect(),
        )
    })
}

fn is_id(field: &str, expected: u32) -> bool {
    field.bytes().all(|byte| byte.is_ascii_digit())
        && field.parse::<u32>().is_ok_and(|id| id == expected)
}

#[cfg(unix)]
fn supervised_output(
    program: &Path,
    args: &[&str],
    timeout: Duration,
) -> Option<Vec<u8>> {
    use std::io::Read as _;
    use std::os::unix::process::CommandExt as _;
    use std::process::{Command, Stdio};

    let mut child = Command::new(program)
        .args(args)
        .env_clear()
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .ok()?;
    let stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let limit = u64::try_from(ANSWER_LIMIT).unwrap_or(u64::MAX) + 1;
        stdout.take(limit).read_to_end(&mut bytes).map(|_| bytes)
    });
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok(None) | Err(_) => break None,
        }
    };
    kill_group(&child);
    let _ = child.wait();
    let bytes = reader.join().ok()?.ok()?;
    (status?.success() && bytes.len() <= ANSWER_LIMIT).then_some(bytes)
}

/// Kills whatever the child left in its group, so nothing holds the pipe the
/// reader waits on.
#[cfg(unix)]
fn kill_group(child: &std::process::Child) {
    use rustix::process::{kill_process_group, Pid, Signal};
    if let Some(group) = i32::try_from(child.id()).ok().and_then(Pid::from_raw)
    {
        let _ = kill_process_group(group, Signal::KILL);
    }
}

#[cfg(not(unix))]
const fn supervised_output(
    _program: &Path,
    _args: &[&str],
    _timeout: Duration,
) -> Option<Vec<u8>> {
    None
}

pub struct LibcLookup;

impl AccountLookup for LibcLookup {
    fn accounts(&self, uid: u32, gid: u32) -> Option<Accounts> {
        let login = libc_db::login(uid)?;
        let (group_name, members) = libc_db::group(gid)?;
        Some(Accounts {
            login,
            group_name,
            members,
        })
    }
}

#[cfg(unix)]
mod libc_db {
    use std::ffi::{c_char, CStr};
    use std::mem::MaybeUninit;
    use std::ptr;

    /// Enough for any account database entry seen in practice; a lookup still
    /// asking for more is treated as having no entry.
    const MAX_BUFFER: usize = 1 << 20;
    const INITIAL_BUFFER: usize = 1 << 10;

    enum Attempt<R> {
        Found(R),
        Absent,
        TooSmall,
    }

    pub fn login(uid: u32) -> Option<String> {
        with_growing_buffer(|buffer| {
            let mut entry = MaybeUninit::<libc::passwd>::uninit();
            let mut found: *mut libc::passwd = ptr::null_mut();
            // SAFETY: every pointer is valid for the call, and `buffer` is
            // passed with its own length, as `getpwuid_r` requires.
            let status = unsafe {
                libc::getpwuid_r(
                    uid,
                    entry.as_mut_ptr(),
                    buffer.as_mut_ptr(),
                    buffer.len(),
                    &raw mut found,
                )
            };
            attempt(status, found, |found| {
                // SAFETY: `found` points at `entry`, whose strings live in
                // `buffer`; both outlive this read.
                unsafe { owned((*found).pw_name) }
            })
        })
    }

    pub fn group(gid: u32) -> Option<(String, Vec<String>)> {
        with_growing_buffer(|buffer| {
            let mut entry = MaybeUninit::<libc::group>::uninit();
            let mut found: *mut libc::group = ptr::null_mut();
            // SAFETY: as for `getpwuid_r` in `login`.
            let status = unsafe {
                libc::getgrgid_r(
                    gid,
                    entry.as_mut_ptr(),
                    buffer.as_mut_ptr(),
                    buffer.len(),
                    &raw mut found,
                )
            };
            attempt(status, found, |found| {
                // SAFETY: as in `login`; `gr_mem` is a null-terminated array
                // of strings inside the same buffer.
                unsafe {
                    let name = owned((*found).gr_name)?;
                    let mut members = Vec::new();
                    let mut member = (*found).gr_mem;
                    while !member.is_null() && !(*member).is_null() {
                        members.push(owned(*member)?);
                        member = member.add(1);
                    }
                    Some((name, members))
                }
            })
        })
    }

    fn attempt<P, R>(
        status: i32,
        found: *mut P,
        read: impl FnOnce(*mut P) -> Option<R>,
    ) -> Attempt<R> {
        match status {
            libc::ERANGE => Attempt::TooSmall,
            0 if !found.is_null() => {
                read(found).map_or(Attempt::Absent, Attempt::Found)
            }
            _ => Attempt::Absent,
        }
    }

    fn with_growing_buffer<R>(
        lookup: impl Fn(&mut [c_char]) -> Attempt<R>,
    ) -> Option<R> {
        let mut size = INITIAL_BUFFER;
        loop {
            let mut buffer = vec![0; size];
            match lookup(&mut buffer) {
                Attempt::Found(found) => return Some(found),
                Attempt::TooSmall if size < MAX_BUFFER => size *= 2,
                Attempt::TooSmall | Attempt::Absent => return None,
            }
        }
    }

    /// # Safety
    ///
    /// `pointer` is null or a live NUL-terminated string.
    unsafe fn owned(pointer: *const c_char) -> Option<String> {
        if pointer.is_null() {
            return None;
        }
        // SAFETY: the caller guarantees a live NUL-terminated string.
        unsafe { CStr::from_ptr(pointer) }
            .to_str()
            .ok()
            .map(str::to_owned)
    }
}

#[cfg(not(unix))]
mod libc_db {
    pub const fn login(_uid: u32) -> Option<String> {
        None
    }

    pub const fn group(_gid: u32) -> Option<(String, Vec<String>)> {
        None
    }
}

#[cfg(all(test, unix))]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    use tempfile::TempDir;

    use super::{
        parse_group_line, parse_passwd_line, AccountLookup as _, Accounts,
        GetentLookup,
    };

    const UID: u32 = 1234;
    const GID: u32 = 1234;

    struct Stub {
        dir: TempDir,
    }

    impl Stub {
        /// Notes that it ran in a marker file, since its environment is
        /// cleared.
        fn new(body: &str) -> Self {
            use std::os::unix::fs::PermissionsExt as _;
            let dir = tempfile::tempdir().expect("stub dir");
            let marker = dir.path().join("ran");
            let script = format!(
                "#!/bin/sh\necho \"$$\" >> '{}'\n{body}\n",
                marker.display()
            );
            let path = dir.path().join("getent");
            std::fs::write(&path, script).expect("stub");
            std::fs::set_permissions(
                &path,
                std::fs::Permissions::from_mode(0o755),
            )
            .expect("executable");
            Self { dir }
        }

        fn path(&self) -> PathBuf {
            self.dir.path().join("getent")
        }

        fn file(&self, name: &str) -> PathBuf {
            self.dir.path().join(name)
        }

        fn pids(&self) -> Vec<i32> {
            std::fs::read_to_string(self.file("ran"))
                .expect("the stub ran")
                .lines()
                .map(|line| line.parse().expect("pid"))
                .collect()
        }

        fn lookup(&self, timeout: Duration) -> GetentLookup {
            GetentLookup::using(
                vec![self.dir.path().join("absent"), self.path()],
                timeout,
            )
        }

        fn accounts(&self) -> Option<Accounts> {
            self.lookup(Duration::from_secs(10)).accounts(UID, GID)
        }
    }

    const ANSWERING: &str = r#"case "$1" in
passwd) printf 'toby:x:%s:1234:Toby:/home/toby:/bin/sh\n' "$2" ;;
group) printf 'toby:x:%s:\n' "$2" ;;
esac"#;

    #[test]
    fn the_getent_fallback_parses_passwd_and_group_lines() {
        assert_eq!(
            parse_passwd_line(
                "toby:x:1234:1234:Toby:/home/toby:/bin/bash",
                UID
            ),
            Some("toby".to_owned())
        );
        assert_eq!(
            parse_group_line("toby:x:1234:", GID),
            Some(("toby".to_owned(), vec![]))
        );
        assert_eq!(
            parse_group_line("devs:*:1234:toby,alex", GID),
            Some((
                "devs".to_owned(),
                vec!["toby".to_owned(), "alex".to_owned()]
            ))
        );
        for line in [
            "toby:x:1235:1234:Toby:/home/toby:/bin/bash",
            "toby:x:+1234:1234:Toby:/home/toby:/bin/bash",
            ":x:1234:1234:Toby:/home/toby:/bin/bash",
            "toby:x:1234",
        ] {
            assert_eq!(parse_passwd_line(line, UID), None, "{line}");
        }
        for line in ["toby:x:1235:", "toby:x:1234", ":x:1234:"] {
            assert_eq!(parse_group_line(line, GID), None, "{line}");
        }
    }

    #[test]
    fn a_getent_answer_for_both_ids_resolves_the_accounts() {
        let stub = Stub::new(ANSWERING);

        assert_eq!(
            stub.accounts(),
            Some(Accounts {
                login: "toby".to_owned(),
                group_name: "toby".to_owned(),
                members: vec![],
            })
        );
        assert_eq!(stub.pids().len(), 2);
    }

    #[test]
    fn the_getent_fallback_rejects_mismatched_or_malformed_answers() {
        let bodies = [
            r"printf 'toby:x:9999:1234:Toby:/home/toby:/bin/sh\n'".to_owned(),
            r#"printf 'toby:x:%s:1234:T:/h:/bin/sh\nalex:x:%s:1:A:/h:/bin/sh\n' "$2" "$2""#
                .to_owned(),
            format!(
                r#"printf 'toby:x:%s:1234:{}:/h:/bin/sh\n' "$2""#,
                "g".repeat(70 * 1024)
            ),
            format!("{ANSWERING}\nexit 2"),
        ];
        for body in bodies {
            let stub = Stub::new(&body);
            assert_eq!(
                stub.accounts(),
                None,
                "{}",
                &body[..body.len().min(80)]
            );
            assert!(!stub.pids().is_empty(), "the stub never ran");
        }
    }

    #[test]
    fn the_getent_fallback_times_out_to_a_refusal() {
        let stub = Stub::new("exec sleep 30");
        let started = Instant::now();

        let accounts =
            stub.lookup(Duration::from_millis(200)).accounts(UID, GID);

        assert_eq!(accounts, None);
        assert!(started.elapsed() < Duration::from_secs(10), "it waited");
        for pid in stub.pids() {
            let pid = rustix::process::Pid::from_raw(pid).expect("pid");
            assert_eq!(
                rustix::process::test_kill_process(pid),
                Err(rustix::io::Errno::SRCH),
                "the stub is still alive"
            );
        }
    }

    #[test]
    fn the_getent_child_runs_with_a_cleared_environment() {
        let stub = Stub::new(&format!(
            "env > \"${{0%/getent}}/environment\"\n{ANSWERING}"
        ));
        stub.accounts();
        let dump = stub.file("environment");

        let environment = std::fs::read_to_string(&dump).expect("dumped");
        let variables: Vec<&str> = environment
            .lines()
            .filter(|line| {
                !["PWD=", "SHLVL=", "_=", "OLDPWD="]
                    .iter()
                    .any(|shell_set| line.starts_with(shell_set))
            })
            .collect();
        assert_eq!(variables, ["LC_ALL=C"], "{environment}");
    }

    #[test]
    fn the_production_getent_executables_are_fixed_absolute_paths() {
        assert_eq!(
            GetentLookup::production().executables(),
            [Path::new("/usr/bin/getent"), Path::new("/bin/getent")]
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_getent_fallback_resolves_the_current_user() {
        let uid = rustix::process::geteuid().as_raw();
        let gid = rustix::process::getegid().as_raw();
        let login = std::process::Command::new("id")
            .arg("-un")
            .output()
            .expect("id");
        let login = String::from_utf8(login.stdout).expect("utf-8");

        let accounts = GetentLookup::production()
            .accounts(uid, gid)
            .expect("getent answers for the current user");

        assert_eq!(accounts.login, login.trim());
    }
}
