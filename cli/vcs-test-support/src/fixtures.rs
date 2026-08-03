//! The checkout fixture matrix: every shape the taxonomy queries must tell
//! apart, built with the real `git` and `jj` binaries beneath a caller-supplied
//! root.
//!
//! The root is supplied rather than owned. A builder holding a
//! `tempfile::TempDir` deletes the tree on drop, which is right in-process and
//! wrong for a CI job that builds the matrix in one step and consumes it in
//! another.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use crate::hermetic::assert_no_repository_ancestor;
use crate::hermetic::Hermetic;
use crate::Error;

/// One (fixture, start directory) pair from the matrix.
#[derive(Debug, Clone)]
pub struct Fixture {
    /// The key used in the recorded oracle mapping.
    pub key: &'static str,
    /// A one-line description of the shape, for failure messages.
    pub shape: &'static str,
    /// The directory queries are invoked from. Canonicalised.
    pub start: PathBuf,
}

/// Accumulates fixtures so each family builder can stay readable.
#[derive(Debug, Default)]
struct Builder(Vec<Fixture>);

impl Builder {
    fn add(&mut self, key: &'static str, shape: &'static str, start: &Path) {
        self.0.push(Fixture {
            key,
            shape,
            start: start.to_path_buf(),
        });
    }
}

/// The repositories more than one fixture hangs off.
///
/// Named per fixture rather than shared. Collapsing them produces two
/// impossible identities: one jj main cannot be both colocated and
/// `--no-colocate`, and one git parent cannot host two linked worktrees both
/// keyed `sub` — git derives the id from the basename and de-duplicates the
/// second to `sub1`, which would also make `worktrees()` count 3 rather than
/// the recorded 1.
#[derive(Debug)]
struct Parents {
    jj_main_colocated: PathBuf,
    jj_main_pure: PathBuf,
    jj_graft_source: PathBuf,
    git_for_graft: PathBuf,
    git_for_nested: PathBuf,
    git_for_nested_pure: PathBuf,
    submodule_seed: PathBuf,
}

/// The built matrix, keyed by the oracle mapping's fixture keys.
#[derive(Debug)]
pub struct Matrix {
    pub base: PathBuf,
    pub fixtures: Vec<Fixture>,
}

impl Matrix {
    /// Builds every fixture beneath `base`.
    ///
    /// # Errors
    ///
    /// When `base` lies inside a repository, or any builder fails.
    pub fn build_in(base: &Path) -> Result<Self, Error> {
        assert_no_repository_ancestor(base)?;
        let base = base.canonicalize()?;
        let env = Hermetic::rooted_at(&base)?;
        let parents = build_parents(&base, &env)?;

        let mut builder = Builder::default();
        add_git_shapes(&mut builder, &base, &env, &parents)?;
        add_jj_shapes(&mut builder, &base, &env, &parents)?;
        add_nested_shapes(&mut builder, &base, &env, &parents)?;
        add_submodule_shapes(&mut builder, &base, &env, &parents)?;
        add_format_shapes(&mut builder, &base, &env)?;
        add_degenerate_shapes(&mut builder, &base)?;

        Ok(Self {
            base,
            fixtures: builder.0,
        })
    }

    /// The start directory recorded for `key`.
    ///
    /// # Errors
    ///
    /// When no fixture carries that key.
    pub fn start(&self, key: &str) -> Result<&Path, Error> {
        self.fixtures
            .iter()
            .find(|fixture| fixture.key == key)
            .map(|fixture| fixture.start.as_path())
            .ok_or_else(|| Error::message(format!("no fixture keyed {key}")))
    }
}

fn build_parents(base: &Path, env: &Hermetic) -> Result<Parents, Error> {
    Ok(Parents {
        jj_main_colocated: colocated(base, "jjmain-colocated", env)?,
        jj_main_pure: pure_jj(&base.join("jjmain-pure"), env)?,
        jj_graft_source: colocated(base, "jjparent", env)?,
        git_for_graft: git_repo(base, "gitparent-cg", env)?,
        git_for_nested: git_repo(base, "gitparent-ngj", env)?,
        git_for_nested_pure: git_repo(base, "gitparent-ngpj", env)?,
        submodule_seed: git_repo(base, "seed", env)?,
    })
}

/// Plain git, the linked worktree from both ends, bare, and no repository.
fn add_git_shapes(
    builder: &mut Builder,
    base: &Path,
    env: &Hermetic,
    parents: &Parents,
) -> Result<(), Error> {
    let plain = git_repo(base, "PG", env)?;
    fs::create_dir_all(plain.join("deep/er"))?;
    builder.add("PG-r", "plain git", &plain);
    builder.add("PG-s", "plain git, subdirectory", &plain.join("deep/er"));

    let worktree_main = git_repo(base, "main", env)?;
    let worktree_linked = base.join("WT");
    env.git(
        &["worktree", "add", "-q", path_arg(&worktree_linked)?],
        &worktree_main,
    )?;
    builder.add(
        "WT-l",
        "linked git worktree, linked",
        &worktree_linked.canonicalize()?,
    );
    builder.add("WT-m", "linked git worktree, main", &worktree_main);

    // Carries neither marker, so the boundary walk cannot reach it.
    let bare_repo = base.join("bare");
    fs::create_dir_all(&bare_repo)?;
    env.git(&["init", "--bare", "--quiet", "."], &bare_repo)?;
    builder.add("BARE", "bare repository", &bare_repo.canonicalize()?);

    let loose = base.join("none");
    fs::create_dir_all(&loose)?;
    builder.add("NONE", "no repository", &loose.canonicalize()?);

    // Classifies as `main`, not `colocated`: that arm also wants a jj secondary
    // workspace and a git linked worktree.
    builder.add("CR", "colocated, real", &colocated(base, "CR", env)?);

    // The only shape classifying as `colocated`: one path that is both.
    let grafted = grafted_colocated(
        base,
        "CG",
        &parents.git_for_graft,
        &parents.jj_graft_source,
        env,
    )?;
    builder.add("CG", "colocated, hand-grafted", &grafted);
    Ok(())
}

/// The jj workspace shapes, colocated and pure.
fn add_jj_shapes(
    builder: &mut Builder,
    base: &Path,
    env: &Hermetic,
    parents: &Parents,
) -> Result<(), Error> {
    let secondary = base.join("JS");
    env.jj(
        &["workspace", "add", path_arg(&secondary)?],
        &parents.jj_main_colocated,
    )?;
    let secondary = secondary.canonicalize()?;
    fs::create_dir_all(secondary.join("deep/er"))?;
    builder.add("JS-r", "jj secondary workspace", &secondary);
    builder.add(
        "JS-s",
        "jj secondary workspace, subdirectory",
        &secondary.join("deep/er"),
    );

    let pure = pure_jj(&base.join("PJ/a/b/c"), env)?;
    builder.add("PJ", "pure jj", &pure);

    let pure_secondary = base.join("PJS");
    env.jj(&["workspace", "add", path_arg(&pure_secondary)?], &pure)?;
    builder.add(
        "PJS",
        "secondary of a pure-jj main",
        &pure_secondary.canonicalize()?,
    );

    // Dual roots differ here while the classification is `jj-secondary`,
    // because the jj and git main roots are equal. This repo's own layout.
    let host = colocated(base, "JSIN", env)?;
    let inner = host.join("workspaces/inner");
    // jj 0.43 does not create intermediate directories.
    fs::create_dir_all(host.join("workspaces"))?;
    env.jj(
        &["workspace", "add", "--name", "inner", path_arg(&inner)?],
        &host,
    )?;
    builder.add(
        "JS-in",
        "jj secondary inside its own colocated main",
        &inner.canonicalize()?,
    );
    Ok(())
}

/// The four nesting shapes, from both ends.
fn add_nested_shapes(
    builder: &mut Builder,
    base: &Path,
    env: &Hermetic,
    parents: &Parents,
) -> Result<(), Error> {
    // The inner directory carries `.jj` only, so a `.git`-inclusive walk sails
    // past it.
    let jj_in_git = git_repo(base, "NJG", env)?;
    let jj_in_git_inner = jj_in_git.join("sub");
    env.jj(
        &["workspace", "add", path_arg(&jj_in_git_inner)?],
        &parents.jj_main_colocated,
    )?;
    builder.add(
        "NJG-i",
        "nested-jj-in-git, inner",
        &jj_in_git_inner.canonicalize()?,
    );
    builder.add("NJG-o", "nested-jj-in-git, outer", &jj_in_git);

    // Only the `.jj`-only walk agrees here: the boundary walk yields `sub`, and
    // the loader reports no repo there.
    let git_in_jj = colocated(base, "NGJ", env)?;
    let git_in_jj_inner = git_in_jj.join("sub");
    env.git(
        &["worktree", "add", "-q", path_arg(&git_in_jj_inner)?],
        &parents.git_for_nested,
    )?;
    builder.add(
        "NGJ-i",
        "nested-git-in-jj, inner",
        &git_in_jj_inner.canonicalize()?,
    );
    builder.add("NGJ-o", "nested-git-in-jj, outer", &git_in_jj);

    let git_in_pure = pure_jj(&base.join("NGPJ"), env)?;
    let git_in_pure_inner = git_in_pure.join("sub");
    env.git(
        &["worktree", "add", "-q", path_arg(&git_in_pure_inner)?],
        &parents.git_for_nested_pure,
    )?;
    builder.add(
        "NGPJ-i",
        "nested-git-in-pure-jj, inner",
        &git_in_pure_inner.canonicalize()?,
    );
    builder.add("NGPJ-o", "nested-git-in-pure-jj, outer", &git_in_pure);

    let pure_in_git = git_repo(base, "PJG", env)?;
    let pure_in_git_inner = pure_in_git.join("sub");
    env.jj(
        &["workspace", "add", path_arg(&pure_in_git_inner)?],
        &parents.jj_main_pure,
    )?;
    builder.add(
        "PJG-i",
        "pure-jj-in-git, inner",
        &pure_in_git_inner.canonicalize()?,
    );
    builder.add("PJG-o", "pure-jj-in-git, outer", &pure_in_git);
    Ok(())
}

/// Every submodule shape, including the three that discriminate the
/// superproject derivation.
fn add_submodule_shapes(
    builder: &mut Builder,
    base: &Path,
    env: &Hermetic,
    parents: &Parents,
) -> Result<(), Error> {
    let seed = &parents.submodule_seed;

    let middle = git_repo(base, "midsrc", env)?;
    add_submodule(env, &middle, seed, "leaf")?;
    let top = git_repo(base, "super", env)?;
    add_submodule(env, &top, &middle, "mid")?;
    env.git(
        &["submodule", "update", "--init", "--recursive", "--quiet"],
        &top,
    )?;
    builder.add("SM-1", "git submodule", &top.join("mid").canonicalize()?);
    // Proves the derivation takes the *nearest* `modules`: the outermost names
    // the superproject, not `mid`.
    builder.add(
        "SM-2",
        "git submodule, depth 2",
        &top.join("mid/leaf").canonicalize()?,
    );
    builder.add("SM-s", "git submodule, superproject root", &top);

    // A nested `.git` *directory*. Reports Kind::Common and no superproject, so
    // a Kind::Submodule-only implementation misses it silently.
    let outer = git_repo(base, "old", env)?;
    let nested = outer.join("inner");
    fs::create_dir_all(&nested)?;
    env.git(&["init", "--quiet"], &nested)?;
    env.git(
        &["commit", "--allow-empty", "--quiet", "-m", "root"],
        &nested,
    )?;
    builder.add("SO", "old-form submodule", &nested.canonicalize()?);

    // Gives `<super>/.git/modules/modules/foo`, where the innermost candidate's
    // parent is not a repository and a bare rposition reports absence.
    let path_clash = git_repo(base, "supm", env)?;
    add_submodule(env, &path_clash, seed, "modules/foo")?;
    builder.add(
        "SM-m",
        "submodule at a path containing `modules`",
        &path_clash.join("modules/foo").canonicalize()?,
    );

    // The dirs differ, so it is a worktree, and git reports no superproject.
    let with_worktree = git_repo(base, "supw", env)?;
    add_submodule(env, &with_worktree, seed, "mid")?;
    let submodule_worktree = base.join("smw");
    env.git(
        &["worktree", "add", "-q", path_arg(&submodule_worktree)?],
        &with_worktree.join("mid"),
    )?;
    builder.add(
        "SM-w",
        "linked worktree of a submodule",
        &submodule_worktree.canonicalize()?,
    );

    // Git puts the git dir under `worktrees/<id>/modules/`, not the common
    // dir's, and reports the *worktree* as the superproject.
    let host = git_repo(base, "supwt", env)?;
    let host_worktree = base.join("supwt-wt");
    env.git(&["worktree", "add", "-q", path_arg(&host_worktree)?], &host)?;
    add_submodule(env, &host_worktree, seed, "sub")?;
    builder.add(
        "SM-wt",
        "submodule inside a linked worktree",
        &host_worktree.join("sub").canonicalize()?,
    );
    Ok(())
}

/// The object-format and ref-backend variants, plus the hostile configuration.
fn add_format_shapes(
    builder: &mut Builder,
    base: &Path,
    env: &Hermetic,
) -> Result<(), Error> {
    builder.add(
        "RF",
        "reftable ref backend",
        &formatted(base, "RF", "ref-format", "reftable", env)?,
    );
    // A sha256 repository's HEAD is 64 hex characters, not 40.
    builder.add(
        "S256",
        "sha256 object format",
        &formatted(base, "S256", "object-format", "sha256", env)?,
    );
    builder.add("HOSTILE", "adversarial configuration", &hostile(base, env)?);
    Ok(())
}

/// The three degenerate shapes. Their expectations are **not** uniform.
fn add_degenerate_shapes(
    builder: &mut Builder,
    base: &Path,
) -> Result<(), Error> {
    let deleted_store = base.join("D1");
    fs::create_dir_all(deleted_store.join(".jj"))?;
    let gone = base.join("D1-store-gone");
    fs::create_dir_all(&gone)?;
    fs::write(deleted_store.join(".jj/repo"), path_arg(&gone)?)?;
    fs::remove_dir_all(&gone)?;
    builder.add(
        "D1",
        "`.jj/repo` -> a deleted directory",
        &deleted_store.canonicalize()?,
    );

    let broken_worktree = base.join("D2");
    fs::create_dir_all(&broken_worktree)?;
    fs::write(
        broken_worktree.join(".git"),
        format!("gitdir: {}\n", base.join("D2-gitdir-gone").display()),
    )?;
    builder.add(
        "D2",
        "`.git` file -> a removed gitdir",
        &broken_worktree.canonicalize()?,
    );

    // `jj workspace root` succeeds here, so only the `<main_root>/.jj/repo`
    // post-condition separates a real answer from a plausible wrong one.
    let wrong_store = base.join("D3");
    fs::create_dir_all(wrong_store.join(".jj"))?;
    let not_a_store = base.join("D3-not-a-store");
    fs::create_dir_all(&not_a_store)?;
    fs::write(wrong_store.join(".jj/repo"), path_arg(&not_a_store)?)?;
    builder.add(
        "D3",
        "`.jj/repo` -> an existing non-store directory",
        &wrong_store.canonicalize()?,
    );
    Ok(())
}

/// The pure-jj benchmark fixture, as a named reusable builder.
///
/// `jj git init` **colocates by default** at 0.43, so `--no-colocate` is
/// mandatory and the absence of `.git` is asserted rather than assumed.
///
/// # Errors
///
/// When jj cannot build the workspace, or it comes out colocated after all.
pub fn pure_jj(root: &Path, env: &Hermetic) -> Result<PathBuf, Error> {
    fs::create_dir_all(root)?;
    env.jj(&["git", "init", "--no-colocate"], root)?;
    env.jj(&["describe", "-m", "root"], root)?;
    env.jj(&["new"], root)?;

    let root = root.canonicalize()?;
    if root.join(".git").exists() {
        return Err(Error::message(format!(
            "{} was built with --no-colocate but carries a .git marker",
            root.display()
        )));
    }
    if !root.join(".jj").is_dir() {
        return Err(Error::message(format!(
            "{} has no .jj directory",
            root.display()
        )));
    }
    Ok(root)
}

/// A plain git repository with one commit, so `git worktree add` works later.
fn git_repo(base: &Path, name: &str, env: &Hermetic) -> Result<PathBuf, Error> {
    let root = base.join(name);
    fs::create_dir_all(&root)?;
    env.git(&["init", "--quiet"], &root)?;
    env.git(&["commit", "--allow-empty", "--quiet", "-m", "root"], &root)?;
    Ok(root.canonicalize()?)
}

fn add_submodule(
    env: &Hermetic,
    host: &Path,
    source: &Path,
    at: &str,
) -> Result<(), Error> {
    env.git(
        &["submodule", "add", "--quiet", path_arg(source)?, at],
        host,
    )?;
    env.git(&["commit", "--quiet", "-m", "add submodule"], host)?;
    Ok(())
}

/// A git repository initialised with one format knob, then asserted to actually
/// carry it — a silently ignored flag would restore the cross-runner drift the
/// pinning exists to remove.
fn formatted(
    base: &Path,
    name: &str,
    knob: &str,
    value: &str,
    env: &Hermetic,
) -> Result<PathBuf, Error> {
    let root = base.join(name);
    fs::create_dir_all(&root)?;
    env.git(&["init", "--quiet", &format!("--{knob}={value}")], &root)?;
    env.git(&["commit", "--allow-empty", "--quiet", "-m", "root"], &root)?;

    let reported = env.git(&["rev-parse", &format!("--show-{knob}")], &root)?;
    if reported != value {
        return Err(Error::message(format!(
            "{name} was initialised with --{knob}={value} but reports {reported}"
        )));
    }
    Ok(root.canonicalize()?)
}

/// A real colocated repository: `git init` then `jj git init --colocate`.
fn colocated(
    base: &Path,
    name: &str,
    env: &Hermetic,
) -> Result<PathBuf, Error> {
    let root = git_repo(base, name, env)?;
    env.jj(&["git", "init", "--colocate"], &root)?;
    Ok(root)
}

/// The hand-grafted colocated shape: one path that is both a git linked
/// worktree and a jj secondary workspace.
///
/// Both `git worktree add` and `jj workspace add` refuse an existing non-empty
/// target, so the jj workspace is built elsewhere and its `.jj` grafted in. The
/// grafted `.jj/repo`'s relative path no longer resolves, so it is rewritten
/// absolute — with **no trailing newline**, because jj reads the file verbatim
/// and a newline turns the target into a nonexistent path.
fn grafted_colocated(
    base: &Path,
    name: &str,
    git_parent: &Path,
    jj_parent: &Path,
    env: &Hermetic,
) -> Result<PathBuf, Error> {
    let target = base.join(name);
    env.git(&["worktree", "add", "-q", path_arg(&target)?], git_parent)?;

    let staging = base.join(format!("{name}-jj-staging"));
    env.jj(&["workspace", "add", path_arg(&staging)?], jj_parent)?;
    fs::rename(staging.join(".jj"), target.join(".jj"))?;
    fs::remove_dir_all(&staging)?;
    fs::write(
        target.join(".jj/repo"),
        path_arg(&jj_parent.join(".jj/repo"))?,
    )?;

    let target = target.canonicalize()?;
    if !target.join(".jj/repo").is_file() {
        return Err(Error::message(format!(
            "{} should carry a .jj/repo *file*",
            target.display()
        )));
    }
    if !target.join(".git").exists() {
        return Err(Error::message(format!(
            "{} should carry a .git marker",
            target.display()
        )));
    }
    Ok(target)
}

/// A plain git repository whose own config points every hook-shaped knob at a
/// marker-writing command.
///
/// None of these ran under the oracle's calls or this story's call set, so the
/// fixture is a regression guard for the APIs that reach gix's filter, pager and
/// external-diff machinery — not evidence about the queries here.
fn hostile(base: &Path, env: &Hermetic) -> Result<PathBuf, Error> {
    let root = git_repo(base, "HOSTILE", env)?;
    let writer = format!(
        "sh -c 'echo ran >> {}'",
        base.join("hostile-marker").display()
    );
    let included = base.join("hostile-include");
    fs::write(&included, format!("[alias]\n\tincluded = !{writer}\n"))?;

    let config = root.join(".git/config");
    let mut text = fs::read_to_string(&config)?;
    write!(
        text,
        "[core]\n\tpager = {writer}\n\tfsmonitor = {writer}\n\
         [diff]\n\texternal = {writer}\n\
         [filter \"hostile\"]\n\tclean = {writer}\n\tsmudge = {writer}\n\
         [alias]\n\thostile = !{writer}\n\
         [include]\n\tpath = {}\n",
        included.display()
    )
    .map_err(|error| Error::message(error.to_string()))?;
    fs::write(&config, text)?;
    Ok(root)
}

fn path_arg(path: &Path) -> Result<&str, Error> {
    path.to_str().ok_or_else(|| {
        Error::message(format!("non-UTF-8 path {}", path.display()))
    })
}
