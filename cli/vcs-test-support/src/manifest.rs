//! The record written beside a shared build, so a later process can adopt the
//! tree without running git or jj.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use crate::fixtures::Fixture;
use crate::Error;

const FILE: &str = "fixtures.tsv";

#[derive(Debug, Clone)]
pub struct Manifest {
    pub fixtures: Vec<Fixture>,
}

impl Manifest {
    /// # Errors
    ///
    /// When the manifest cannot be written beneath `base`.
    pub fn write(&self, base: &Path) -> Result<(), Error> {
        let mut text = String::new();
        for fixture in &self.fixtures {
            writeln!(
                text,
                "{}\t{}\t{}",
                fixture.key,
                fixture.shape,
                fixture.start.display()
            )
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
        let fixtures = fs::read_to_string(&path)?
            .lines()
            .map(parse_line)
            .collect::<Result<Vec<_>, _>>()?;
        if fixtures.is_empty() {
            return Err(Error::message(format!(
                "the fixture manifest at {} is empty",
                path.display()
            )));
        }
        Ok(Self { fixtures })
    }
}

fn parse_line(line: &str) -> Result<Fixture, Error> {
    let mut fields = line.split('\t');
    let (Some(key), Some(shape), Some(start), None) =
        (fields.next(), fields.next(), fields.next(), fields.next())
    else {
        return Err(Error::message(format!(
            "malformed fixture manifest line: {line}"
        )));
    };
    Ok(Fixture {
        key: key.to_owned(),
        shape: shape.to_owned(),
        start: PathBuf::from(start),
    })
}
