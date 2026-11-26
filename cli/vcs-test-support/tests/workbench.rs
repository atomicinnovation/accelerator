use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use serde_json::Value;
use vcs_test_support::family::CopyableFamily;
use vcs_test_support::family::Family;
use vcs_test_support::family::PrivateBuild;
use vcs_test_support::family::SharedFamily;
use vcs_test_support::fixture_log::FixtureLog;
use vcs_test_support::fixture_log::TestName;
use vcs_test_support::fixture_log::FIXTURE_LOG_VARIABLE;
use vcs_test_support::handover::Handover;
use vcs_test_support::handover::SHARED_WORKBENCHES_VARIABLE;
use vcs_test_support::hermetic::Hermetic;
use vcs_test_support::workbench::Workbench;
use vcs_test_support::Error;

type TestError = Box<dyn std::error::Error>;

const MARKER: &str = "marker";
const PART: &str = "part";
const PART_FILE: &str = "part/file";

/// A family that costs a few file writes, so no test here runs git or jj.
#[derive(Debug)]
struct Marked {
    root: PathBuf,
    built_with_home: Option<String>,
    part: Option<PathBuf>,
}

/// The one part of [`Marked`] a test can take a writable copy of.
struct ThePart;

impl SharedFamily for Marked {
    const FAMILY: Family = Family::Matrix;
    type Inputs = ();

    fn adopt(tree: &Path) -> Result<Self, Error> {
        if !tree.join(MARKER).is_file() {
            return Err(Error::message(format!(
                "{} carries no marker",
                tree.display()
            )));
        }
        Ok(Self {
            root: tree.to_path_buf(),
            built_with_home: None,
            part: Some(tree.join(PART)),
        })
    }

    fn build_privately(
        directory: &Path,
        hermetic: &Hermetic,
        _inputs: &(),
        _token: PrivateBuild,
    ) -> Result<Self, Error> {
        let home = hermetic.home().display().to_string();
        fs::write(directory.join(MARKER), &home)?;
        Ok(Self {
            root: directory.to_path_buf(),
            built_with_home: Some(home),
            part: None,
        })
    }
}

impl CopyableFamily for Marked {
    type Part = ThePart;

    fn entries(_part: &ThePart, _tree: &Path) -> Result<Vec<String>, Error> {
        Ok(vec![PART.to_owned()])
    }

    fn build_part_privately(
        _part: ThePart,
        directory: &Path,
        hermetic: &Hermetic,
        _inputs: &(),
        _token: PrivateBuild,
    ) -> Result<Self, Error> {
        fs::create_dir_all(directory.join(PART))?;
        fs::write(directory.join(PART_FILE), "private")?;
        Ok(Self {
            root: directory.to_path_buf(),
            built_with_home: Some(hermetic.home().display().to_string()),
            part: Some(directory.join(PART)),
        })
    }

    fn rebased(self, from: &Path, onto: &Path, entries: &[String]) -> Self {
        Self {
            root: onto.to_path_buf(),
            built_with_home: self.built_with_home,
            part: self
                .part
                .and_then(|part| {
                    part.strip_prefix(from).ok().map(Path::to_path_buf)
                })
                .filter(|relative| {
                    entries.iter().any(|entry| relative.starts_with(entry))
                })
                .map(|relative| onto.join(relative)),
        }
    }
}

/// [`Marked`], but its part's file holds what the build is given.
#[derive(Debug)]
struct MarkedWith(Marked);

/// What a private build of [`MarkedWith`] writes into its part's file.
struct PartContents(&'static str);

impl SharedFamily for MarkedWith {
    const FAMILY: Family = Family::Matrix;
    type Inputs = PartContents;

    fn adopt(tree: &Path) -> Result<Self, Error> {
        Marked::adopt(tree).map(Self)
    }

    fn build_privately(
        directory: &Path,
        hermetic: &Hermetic,
        _inputs: &PartContents,
        token: PrivateBuild,
    ) -> Result<Self, Error> {
        Marked::build_privately(directory, hermetic, &(), token).map(Self)
    }
}

impl CopyableFamily for MarkedWith {
    type Part = ThePart;

    fn entries(part: &ThePart, tree: &Path) -> Result<Vec<String>, Error> {
        Marked::entries(part, tree)
    }

    fn build_part_privately(
        part: ThePart,
        directory: &Path,
        hermetic: &Hermetic,
        inputs: &PartContents,
        token: PrivateBuild,
    ) -> Result<Self, Error> {
        let built = Marked::build_part_privately(
            part,
            directory,
            hermetic,
            &(),
            token,
        )?;
        fs::write(directory.join(PART_FILE), inputs.0)?;
        Ok(Self(built))
    }

    fn rebased(self, from: &Path, onto: &Path, entries: &[String]) -> Self {
        Self(self.0.rebased(from, onto, entries))
    }
}

struct Run {
    directory: tempfile::TempDir,
}

impl Run {
    fn new() -> Result<Self, TestError> {
        Ok(Self {
            directory: tempfile::tempdir()?,
        })
    }

    fn root(&self) -> Result<PathBuf, TestError> {
        Ok(self.directory.path().canonicalize()?)
    }

    fn log_path(&self) -> Result<PathBuf, TestError> {
        Ok(self.root()?.join("run.jsonl"))
    }

    fn shared_tree(&self) -> Result<PathBuf, TestError> {
        self.shared_tree_whose_part_holds("shared")
    }

    fn shared_tree_whose_part_holds(
        &self,
        contents: &str,
    ) -> Result<PathBuf, TestError> {
        let tree = self.root()?.join("matrix");
        fs::create_dir_all(tree.join(PART))?;
        fs::write(tree.join(MARKER), "shared")?;
        fs::write(tree.join(PART_FILE), contents)?;
        Ok(tree)
    }

    fn workbenches(&self) -> Result<PathBuf, TestError> {
        let workbenches = self.root()?.join("workbenches");
        fs::create_dir_all(&workbenches)?;
        Ok(workbenches)
    }

    fn handing_nothing(&self) -> Result<Handover, TestError> {
        Ok(Handover {
            log: FixtureLog::at(self.log_path()?),
            ..Handover::nothing()
        })
    }

    fn handing_over(&self, tree: PathBuf) -> Result<Handover, TestError> {
        Ok(Handover {
            trees: BTreeMap::from([(Family::Matrix, tree)]),
            workbenches: Some(self.workbenches()?),
            ..self.handing_nothing()?
        })
    }

    fn events(&self) -> Result<Vec<Value>, TestError> {
        let path = self.log_path()?;
        if !path.exists() {
            return Ok(Vec::new());
        }
        fs::read_to_string(path)?
            .lines()
            .map(|line| serde_json::from_str(line).map_err(Into::into))
            .collect()
    }
}

fn lookup(variables: &[(&str, &Path)]) -> impl Fn(&str) -> Option<String> {
    let variables: BTreeMap<String, String> = variables
        .iter()
        .map(|(name, path)| ((*name).to_owned(), path.display().to_string()))
        .collect();
    move |name| variables.get(name).cloned()
}

#[test]
fn a_workbench_opens_under_the_handed_workbenches_directory(
) -> Result<(), TestError> {
    let run = Run::new()?;
    let tree = run.shared_tree()?;
    let workbench = Workbench::with(run.handing_over(tree.clone())?)?;

    assert_eq!(
        workbench.path().parent(),
        Some(run.workbenches()?.as_path())
    );
    assert_eq!(workbench.path(), workbench.path().canonicalize()?);
    assert!(!workbench.path().starts_with(&tree));
    Ok(())
}

#[test]
fn a_workbench_opens_under_the_system_temp_directory_by_default(
) -> Result<(), TestError> {
    let run = Run::new()?;
    let workbench = Workbench::with(run.handing_nothing()?)?;

    assert!(workbench
        .path()
        .starts_with(env::temp_dir().canonicalize()?));
    assert_eq!(workbench.path(), workbench.path().canonicalize()?);
    Ok(())
}

#[test]
fn the_hermetic_environment_is_rooted_in_the_workbench() -> Result<(), TestError>
{
    let run = Run::new()?;
    let workbench = Workbench::with(run.handing_nothing()?)?;

    assert!(workbench.hermetic().home().starts_with(workbench.path()));
    Ok(())
}

#[test]
fn two_workbenches_have_distinct_hermetic_roots() -> Result<(), TestError> {
    let run = Run::new()?;
    let first = Workbench::with(run.handing_nothing()?)?;
    let second = Workbench::with(run.handing_nothing()?)?;

    assert_ne!(first.hermetic().home(), second.hermetic().home());
    Ok(())
}

#[test]
fn with_nothing_handed_the_family_is_built_privately_in_the_workbench(
) -> Result<(), TestError> {
    let run = Run::new()?;
    let workbench = Workbench::with(run.handing_nothing()?)?;

    let marked = workbench.shared::<Marked>()?;

    assert_eq!(marked.root, workbench.path());
    assert_eq!(
        marked.built_with_home.as_deref(),
        workbench.hermetic().home().to_str()
    );
    let events = run.events()?;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event"], "private-build");
    assert_eq!(events[0]["fixtures"], "matrix");
    let current = test_name();
    assert_eq!(events[0]["binary"], current["binary"]);
    assert_eq!(events[0]["test"], current["test"]);
    assert_eq!(
        events[0]["hermetic-root"],
        Value::String(workbench.path().display().to_string())
    );
    Ok(())
}

#[test]
fn a_handed_tree_is_adopted_and_nothing_is_built() -> Result<(), TestError> {
    let run = Run::new()?;
    let tree = run.shared_tree()?;
    let workbench = Workbench::with(run.handing_over(tree.clone())?)?;

    let marked = workbench.shared::<Marked>()?;

    assert_eq!(marked.root, tree);
    assert_eq!(marked.built_with_home, None);
    assert!(!workbench.path().join(MARKER).exists());
    let events = run.events()?;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event"], "adoption");
    assert_eq!(events[0]["family"], "matrix");
    assert_eq!(
        events[0]["hermetic-root"],
        Value::String(workbench.path().display().to_string())
    );
    Ok(())
}

#[test]
fn a_missing_handed_tree_fails_naming_the_variable_and_the_path(
) -> Result<(), TestError> {
    let run = Run::new()?;
    let missing = run.root()?.join("gone");
    let workbench = Workbench::with(run.handing_over(missing.clone())?)?;

    let error = workbench.shared::<Marked>().err().ok_or("it should fail")?;

    assert_names_the_handover(&error, &missing);
    assert!(run.events()?.is_empty());
    assert!(!workbench.path().join(MARKER).exists());
    Ok(())
}

#[test]
fn an_unadoptable_handed_tree_fails_naming_the_variable_and_the_path(
) -> Result<(), TestError> {
    let run = Run::new()?;
    let unmarked = run.root()?.join("unmarked");
    fs::create_dir_all(&unmarked)?;
    let workbench = Workbench::with(run.handing_over(unmarked.clone())?)?;

    let error = workbench.shared::<Marked>().err().ok_or("it should fail")?;

    assert_names_the_handover(&error, &unmarked);
    assert!(run.events()?.is_empty());
    assert!(!workbench.path().join(MARKER).exists());
    Ok(())
}

fn assert_names_the_handover(error: &Error, path: &Path) {
    let message = error.to_string();
    assert!(
        message.contains(Family::Matrix.handover_variable()),
        "{message}"
    );
    assert!(message.contains(&path.display().to_string()), "{message}");
}

#[test]
fn handed_paths_are_canonicalised() -> Result<(), TestError> {
    let run = Run::new()?;
    let tree = run.shared_tree()?;
    let workbenches = run.workbenches()?;
    let tree_alias = run.root()?.join("matrix-alias");
    let workbenches_alias = run.root()?.join("workbenches-alias");
    std::os::unix::fs::symlink(&tree, &tree_alias)?;
    std::os::unix::fs::symlink(&workbenches, &workbenches_alias)?;
    let log = run.log_path()?;

    let handover = Handover::from_variables(lookup(&[
        (Family::Matrix.handover_variable(), &tree_alias),
        (SHARED_WORKBENCHES_VARIABLE, &workbenches_alias),
        (FIXTURE_LOG_VARIABLE, &log),
    ]));
    assert_eq!(handover.shared_tree(Family::Matrix), Some(tree.as_path()));
    assert_eq!(handover.workbenches.as_deref(), Some(workbenches.as_path()));

    let workbench = Workbench::with(handover)?;
    let marked = workbench.shared::<Marked>()?;

    assert_eq!(marked.root, tree);
    assert!(workbench.path().starts_with(&workbenches));
    assert_eq!(
        run.events()?[0]["hermetic-root"],
        Value::String(workbench.path().display().to_string())
    );
    Ok(())
}

#[test]
fn a_handed_path_that_does_not_exist_is_kept_as_given() -> Result<(), TestError>
{
    let run = Run::new()?;
    let missing = run.root()?.join("gone");

    let handover = Handover::from_variables(lookup(&[(
        Family::Matrix.handover_variable(),
        &missing,
    )]));

    assert_eq!(
        handover.shared_tree(Family::Matrix),
        Some(missing.as_path())
    );
    Ok(())
}

#[test]
fn an_empty_variable_is_unset() {
    let empty = Path::new("");

    let handover = Handover::from_variables(lookup(&[
        (Family::Matrix.handover_variable(), empty),
        (SHARED_WORKBENCHES_VARIABLE, empty),
        (FIXTURE_LOG_VARIABLE, empty),
    ]));

    assert_eq!(handover.shared_tree(Family::Matrix), None);
    assert_eq!(handover.workbenches, None);
    assert_eq!(handover.log.location(), None);
}

#[test]
fn the_test_is_named_from_the_variables() {
    let handover = Handover::from_variables(|name| match name {
        "NEXTEST_BINARY_ID" => Some("vcs-adapters::classify".to_owned()),
        "NEXTEST_TEST_NAME" => Some("a_test".to_owned()),
        _ => None,
    });

    assert_eq!(
        serde_json::to_value(&handover.test).ok(),
        Some(serde_json::json!({
            "binary": "vcs-adapters::classify",
            "test": "a_test",
        }))
    );
}

struct VariableSetForThisTest {
    name: &'static str,
    previous: Option<OsString>,
}

impl VariableSetForThisTest {
    fn new(name: &'static str, value: &Path) -> Self {
        let previous = env::var_os(name);
        env::set_var(name, value);
        Self { name, previous }
    }
}

impl Drop for VariableSetForThisTest {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => env::set_var(self.name, value),
            None => env::remove_var(self.name),
        }
    }
}

#[test]
fn the_environment_is_read_from_the_process() -> Result<(), TestError> {
    let workbenches = tempfile::tempdir()?;
    let _set = VariableSetForThisTest::new(
        SHARED_WORKBENCHES_VARIABLE,
        workbenches.path(),
    );

    let handover = Handover::from_environment();

    assert_eq!(
        handover.workbenches,
        Some(workbenches.path().canonicalize()?)
    );
    Ok(())
}

#[test]
fn a_workbench_hands_out_one_fixture() -> Result<(), TestError> {
    let run = Run::new()?;
    let tree = run.shared_tree()?;
    let workbench = Workbench::with(run.handing_over(tree)?)?;
    let _first = workbench.shared::<Marked>()?;

    let error = workbench.shared::<Marked>().err().ok_or("it should fail")?;

    assert_eq!(
        error.to_string(),
        "this workbench already holds the matrix (adopted); open another \
         workbench for the matrix"
    );
    assert_eq!(run.events()?.len(), 1);
    Ok(())
}

#[test]
fn a_privately_built_fixture_is_named_as_built() -> Result<(), TestError> {
    let run = Run::new()?;
    let workbench = Workbench::with(run.handing_nothing()?)?;
    let _first = workbench.shared::<Marked>()?;

    let error = workbench.shared::<Marked>().err().ok_or("it should fail")?;

    assert!(
        error.to_string().contains("holds the matrix (built)"),
        "{error}"
    );
    Ok(())
}

#[test]
fn dropping_the_workbench_removes_its_directory() -> Result<(), TestError> {
    let run = Run::new()?;
    let workbench = Workbench::with(run.handing_nothing()?)?;
    workbench.shared::<Marked>()?;
    let path = workbench.path().to_path_buf();

    drop(workbench);

    assert!(!path.exists());
    Ok(())
}

fn test_name() -> Value {
    serde_json::to_value(vcs_test_support::fixture_log::TestName::current())
        .unwrap_or_default()
}

#[test]
fn a_hand_over_of_nothing_names_only_the_current_test() {
    let handover = Handover::nothing();

    assert!(handover.trees.is_empty());
    assert_eq!(handover.workbenches, None);
    assert_eq!(handover.log.location(), None);
    assert_eq!(handover.test, TestName::current());
}

#[test]
fn a_writable_part_of_a_handed_tree_is_copied_into_the_workbench(
) -> Result<(), TestError> {
    let run = Run::new()?;
    let workbench = Workbench::with(run.handing_over(run.shared_tree()?)?)?;

    let marked = workbench.writable::<Marked>(ThePart)?;

    assert_eq!(marked.root, workbench.path());
    assert_eq!(marked.part, Some(workbench.path().join(PART)));
    assert_eq!(
        fs::read_to_string(workbench.path().join(PART_FILE))?,
        "shared"
    );
    assert!(!workbench.path().join(MARKER).exists());
    let events = run.events()?;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event"], "copy");
    assert_eq!(events[0]["family"], "matrix");
    let workbench_path = Value::String(workbench.path().display().to_string());
    assert_eq!(events[0]["hermetic-root"], workbench_path);
    assert_eq!(events[0]["copy"], workbench_path);
    assert_eq!(events[0]["named-by"], Value::Array(Vec::new()));
    Ok(())
}

#[test]
fn a_writable_part_with_nothing_handed_is_built_privately(
) -> Result<(), TestError> {
    let run = Run::new()?;
    let workbench = Workbench::with(run.handing_nothing()?)?;

    let marked = workbench.writable::<Marked>(ThePart)?;

    assert_eq!(marked.part, Some(workbench.path().join(PART)));
    assert_eq!(
        marked.built_with_home.as_deref(),
        workbench.hermetic().home().to_str()
    );
    let events = run.events()?;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event"], "private-build");
    assert_eq!(events[0]["fixtures"], "matrix");
    Ok(())
}

#[test]
fn a_copy_still_naming_its_shared_tree_is_recorded_and_fails(
) -> Result<(), TestError> {
    let run = Run::new()?;
    let tree = run.root()?.join("matrix");
    let tree = run.shared_tree_whose_part_holds(&format!(
        "origin = {}/elsewhere",
        tree.display()
    ))?;
    let workbench = Workbench::with(run.handing_over(tree.clone())?)?;

    let error = workbench
        .writable::<Marked>(ThePart)
        .err()
        .ok_or("it should fail")?;

    let copied_file = workbench.path().join(PART_FILE);
    let message = error.to_string();
    assert!(
        message.contains(&copied_file.display().to_string()),
        "{message}"
    );
    assert!(message.contains(&tree.display().to_string()), "{message}");
    let events = run.events()?;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event"], "copy");
    assert_eq!(
        events[0]["named-by"],
        serde_json::json!([copied_file.display().to_string()])
    );
    Ok(())
}

fn a_private_set(
    directory: &Path,
    hermetic: &Hermetic,
) -> Result<BTreeMap<&'static str, PathBuf>, Error> {
    let state = directory.join("private-state");
    fs::create_dir_all(&state)?;
    fs::write(state.join("home"), hermetic.home().display().to_string())?;
    Ok(BTreeMap::from([("private-state", state)]))
}

#[test]
fn a_private_set_is_built_in_the_workbench_and_logged_as_one(
) -> Result<(), TestError> {
    let run = Run::new()?;
    let workbench = Workbench::with(run.handing_over(run.shared_tree()?)?)?;

    let states = workbench.private_states(a_private_set)?;

    let state = states.get("private-state").ok_or("no private state")?;
    assert_eq!(state, &workbench.path().join("private-state"));
    assert_eq!(
        fs::read_to_string(state.join("home"))?,
        workbench.hermetic().home().display().to_string()
    );
    let events = run.events()?;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event"], "private-build");
    assert_eq!(events[0]["fixtures"], "private-sets");
    assert_eq!(
        events[0]["hermetic-root"],
        Value::String(workbench.path().display().to_string())
    );
    Ok(())
}

#[test]
fn a_workbench_holding_an_adoption_refuses_a_copy() -> Result<(), TestError> {
    let run = Run::new()?;
    let workbench = Workbench::with(run.handing_over(run.shared_tree()?)?)?;
    let _first = workbench.shared::<Marked>()?;

    let error = workbench
        .writable::<Marked>(ThePart)
        .err()
        .ok_or("it should fail")?;

    assert_eq!(
        error.to_string(),
        "this workbench already holds the matrix (adopted); open another \
         workbench for the matrix"
    );
    Ok(())
}

#[test]
fn a_workbench_holding_a_copy_refuses_a_private_set() -> Result<(), TestError> {
    let run = Run::new()?;
    let workbench = Workbench::with(run.handing_over(run.shared_tree()?)?)?;
    let _first = workbench.writable::<Marked>(ThePart)?;

    let error = workbench
        .private_states(a_private_set)
        .err()
        .ok_or("it should fail")?;

    assert_eq!(
        error.to_string(),
        "this workbench already holds the matrix (copied); open another \
         workbench for the private sets"
    );
    Ok(())
}

#[test]
fn a_workbench_holding_a_private_set_refuses_an_adoption(
) -> Result<(), TestError> {
    let run = Run::new()?;
    let workbench = Workbench::with(run.handing_over(run.shared_tree()?)?)?;
    let _first = workbench.private_states(a_private_set)?;

    let error = workbench.shared::<Marked>().err().ok_or("it should fail")?;

    assert_eq!(
        error.to_string(),
        "this workbench already holds the private sets (built); open \
         another workbench for the matrix"
    );
    assert_eq!(run.events()?.len(), 1);
    Ok(())
}

#[test]
fn a_writable_part_needing_inputs_is_built_privately_with_them(
) -> Result<(), TestError> {
    let run = Run::new()?;
    let workbench = Workbench::with(run.handing_nothing()?)?;

    let marked = workbench
        .writable_with::<MarkedWith>(ThePart, &PartContents("given"))?;

    assert_eq!(marked.0.part, Some(workbench.path().join(PART)));
    assert_eq!(
        fs::read_to_string(workbench.path().join(PART_FILE))?,
        "given"
    );
    let events = run.events()?;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event"], "private-build");
    assert_eq!(events[0]["fixtures"], "matrix");
    Ok(())
}

#[test]
fn a_writable_part_needing_inputs_is_copied_without_them(
) -> Result<(), TestError> {
    let run = Run::new()?;
    let workbench = Workbench::with(run.handing_over(run.shared_tree()?)?)?;

    let marked = workbench
        .writable_with::<MarkedWith>(ThePart, &PartContents("given"))?;

    assert_eq!(marked.0.part, Some(workbench.path().join(PART)));
    assert_eq!(
        fs::read_to_string(workbench.path().join(PART_FILE))?,
        "shared"
    );
    let events = run.events()?;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event"], "copy");
    Ok(())
}
