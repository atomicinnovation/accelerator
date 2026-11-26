//! The record written beside a shared stalled-migration build, so a later
//! process can adopt the stalls without running jj or migrate.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use crate::stalled_migration::Stall;
use crate::stalled_migration::StallKey;
use crate::Error;

const FILE: &str = "stalled-migrations.tsv";

#[derive(Debug, Clone)]
pub struct StalledMigrationManifest {
    pub stalls: BTreeMap<StallKey, Stall>,
}

impl StalledMigrationManifest {
    /// # Errors
    ///
    /// When the manifest cannot be written beneath `base`.
    pub fn write(&self, base: &Path) -> Result<(), Error> {
        let mut text = String::new();
        for (key, stall) in &self.stalls {
            writeln!(text, "{key}\t{}\t{}", stall.base, stall.sibling)
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
        let stalls = fs::read_to_string(&path)?
            .lines()
            .map(|line| parse_line(base, line))
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        if stalls.is_empty() {
            return Err(Error::message(format!(
                "the stalled-migration manifest at {} is empty",
                path.display()
            )));
        }
        Ok(Self { stalls })
    }
}

fn parse_line(build: &Path, line: &str) -> Result<(StallKey, Stall), Error> {
    let mut fields = line.split('\t');
    let (Some(key), Some(base), Some(sibling), None) =
        (fields.next(), fields.next(), fields.next(), fields.next())
    else {
        return Err(Error::message(format!(
            "malformed stalled-migration manifest line: {line}"
        )));
    };
    let key: StallKey = key.parse()?;
    Ok((
        key,
        Stall {
            root: build.join(key.to_string()),
            base: base.to_owned(),
            sibling: sibling.to_owned(),
        },
    ))
}
