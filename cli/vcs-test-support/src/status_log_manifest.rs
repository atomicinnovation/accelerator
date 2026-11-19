//! The record written beside a shared status-log build, so a later process can
//! adopt the states without running git or jj.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use crate::Error;

const FILE: &str = "status-log-states.tsv";

#[derive(Debug, Clone)]
pub struct StatusLogManifest {
    pub states: BTreeMap<String, PathBuf>,
}

impl StatusLogManifest {
    /// # Errors
    ///
    /// When the manifest cannot be written beneath `base`.
    pub fn write(&self, base: &Path) -> Result<(), Error> {
        let mut text = String::new();
        for (key, path) in &self.states {
            writeln!(text, "{key}\t{}", path.display())
                .map_err(|error| Error::message(error.to_string()))?;
        }
        fs::write(base.join(FILE), text)?;
        Ok(())
    }

    /// # Errors
    ///
    /// When `base` holds no manifest, or it is malformed or empty.
    pub fn read(base: &Path) -> Result<Self, Error> {
        let path = base.join(FILE);
        if !path.is_file() {
            return Err(Error::message(format!(
                "{} holds no {FILE}, so it is not a shared build",
                base.display()
            )));
        }
        let states = fs::read_to_string(&path)?
            .lines()
            .map(parse_line)
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        if states.is_empty() {
            return Err(Error::message(format!(
                "the status-log manifest at {} is empty",
                path.display()
            )));
        }
        Ok(Self { states })
    }

    pub(crate) fn exists_in(base: &Path) -> bool {
        base.join(FILE).is_file()
    }
}

fn parse_line(line: &str) -> Result<(String, PathBuf), Error> {
    let mut fields = line.split('\t');
    let (Some(key), Some(path), None) =
        (fields.next(), fields.next(), fields.next())
    else {
        return Err(Error::message(format!(
            "malformed status-log manifest line: {line}"
        )));
    };
    Ok((key.to_owned(), PathBuf::from(path)))
}
