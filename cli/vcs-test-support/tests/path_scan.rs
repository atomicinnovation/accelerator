use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::path::PathBuf;

use tempfile::TempDir;
use vcs_test_support::path_scan::files_naming;

type TestError = Box<dyn std::error::Error>;

struct TwoSpellings {
    _directory: TempDir,
    canonical: PathBuf,
    through_link: PathBuf,
}

fn a_directory_with_two_spellings() -> Result<TwoSpellings, TestError> {
    let directory = tempfile::tempdir()?;
    let canonical = directory.path().canonicalize()?.join("real");
    fs::create_dir(&canonical)?;
    let through_link = directory.path().join("link");
    symlink(&canonical, &through_link)?;
    Ok(TwoSpellings {
        _directory: directory,
        canonical,
        through_link,
    })
}

fn a_tree() -> Result<TempDir, TestError> {
    let tree = tempfile::tempdir()?;
    fs::create_dir_all(tree.path().join("nested/deeper"))?;
    fs::write(tree.path().join("nested/deeper/innocent"), "nothing here\n")?;
    Ok(tree)
}

fn mentioning(path: &Path) -> String {
    format!(
        "[remote \"origin\"]\n\turl = {}/origin.git\n",
        path.display()
    )
}

#[test]
fn a_tree_that_does_not_name_the_needle_has_no_files_naming_it(
) -> Result<(), TestError> {
    let spellings = a_directory_with_two_spellings()?;
    let tree = a_tree()?;

    assert_eq!(
        files_naming(tree.path(), &spellings.through_link)?,
        Vec::<PathBuf>::new()
    );
    Ok(())
}

#[test]
fn a_file_naming_the_needle_as_given_is_found() -> Result<(), TestError> {
    let spellings = a_directory_with_two_spellings()?;
    let tree = a_tree()?;
    let config = tree.path().join("nested/config");
    fs::write(&config, mentioning(&spellings.through_link))?;

    assert_eq!(
        files_naming(tree.path(), &spellings.through_link)?,
        vec![config]
    );
    Ok(())
}

#[test]
fn a_file_naming_the_needles_canonical_spelling_is_found(
) -> Result<(), TestError> {
    let spellings = a_directory_with_two_spellings()?;
    let tree = a_tree()?;
    let config = tree.path().join("nested/deeper/config");
    fs::write(&config, mentioning(&spellings.canonical))?;

    assert_eq!(
        files_naming(tree.path(), &spellings.through_link)?,
        vec![config]
    );
    Ok(())
}

#[test]
fn a_symlink_whose_target_names_the_needle_is_found() -> Result<(), TestError> {
    let spellings = a_directory_with_two_spellings()?;
    let tree = a_tree()?;
    let link = tree.path().join("nested/pointer");
    symlink(spellings.canonical.join("elsewhere"), &link)?;

    assert_eq!(
        files_naming(tree.path(), &spellings.through_link)?,
        vec![link]
    );
    Ok(())
}

#[test]
fn every_file_naming_the_needle_is_found_in_path_order() -> Result<(), TestError>
{
    let spellings = a_directory_with_two_spellings()?;
    let tree = a_tree()?;
    let first = tree.path().join("a-config");
    let second = tree.path().join("nested/deeper/z-config");
    fs::write(&second, mentioning(&spellings.canonical))?;
    fs::write(&first, mentioning(&spellings.through_link))?;

    assert_eq!(
        files_naming(tree.path(), &spellings.through_link)?,
        vec![first, second]
    );
    Ok(())
}
