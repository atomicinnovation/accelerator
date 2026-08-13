//! Launcher-side term decomposition for the warm-dispatch measurement.
//!
//! Reports the two warm-path terms that are only reachable from inside the
//! crate — `cache::find`'s directory scan and the cache-hit re-verification —
//! as JSON on stdout, so `tasks/measure.py` can close its composition budget
//! against figures taken in the same session as the confirmatory `G` rather
//! than importing them across sessions.
//!
//! `#[ignore]` by design: it reads the operator's live cache root and reports
//! timings rather than asserting a threshold, so it is meaningless on a CI
//! runner and must never gate one. Run it through
//! `mise run measure:warm-dispatch`, which invokes it explicitly.
//!
//! The re-verification body below **replicates** the resolver's private
//! `reverify`, which no public surface exposes. Re-read
//! `cli/launcher/src/launch/outbound/resolve/mod.rs` at the revision of any
//! re-measurement before trusting the figure: a refactor of that method would
//! silently invalidate the replica, and nothing here would fail.

use std::path::{Path, PathBuf};
use std::time::Instant;

use accelerator::launch::outbound::resolve::{
    cache, keys::TrustedKeys, verifier,
};

type TestError = Box<dyn std::error::Error>;

const SAMPLES: usize = 200;

fn cache_root() -> Result<PathBuf, TestError> {
    let root = std::env::var("ACCELERATOR_MEASURE_CACHE_ROOT")
        .map_err(|_| "set ACCELERATOR_MEASURE_CACHE_ROOT to the cache root")?;
    Ok(PathBuf::from(root))
}

fn subbinary() -> String {
    std::env::var("ACCELERATOR_MEASURE_SUBBINARY")
        .unwrap_or_else(|_| "vcs".to_owned())
}

fn version() -> Result<String, TestError> {
    Ok(std::env::var("ACCELERATOR_MEASURE_VERSION")
        .map_err(|_| "set ACCELERATOR_MEASURE_VERSION to the tree's version")?)
}

/// Median of `samples` in milliseconds, matching the harness's convention.
fn median_ms(mut samples: Vec<f64>) -> f64 {
    samples.sort_by(f64::total_cmp);
    let middle = samples.len() / 2;
    if samples.len().is_multiple_of(2) {
        f64::midpoint(samples[middle - 1], samples[middle])
    } else {
        samples[middle]
    }
}

/// Linearly interpolated percentile, the quantile given as a whole fraction.
///
/// Integer index arithmetic rather than a float position: the harness pins
/// linear interpolation as the recorded convention, and a float round trip
/// through the index would put a cast in the one place the two sides must
/// agree exactly.
fn percentile_ms(
    mut samples: Vec<f64>,
    numerator: u32,
    denominator: u32,
) -> f64 {
    samples.sort_by(f64::total_cmp);
    let span = u32::try_from(samples.len() - 1).unwrap_or(u32::MAX);
    let scaled = u64::from(span) * u64::from(numerator);
    let low = usize::try_from(scaled / u64::from(denominator)).unwrap_or(0);
    let remainder = scaled % u64::from(denominator);
    if remainder == 0 || low + 1 >= samples.len() {
        return samples[low];
    }
    let fraction = u32::try_from(remainder).map_or(0.0, f64::from)
        / f64::from(denominator);
    fraction.mul_add(samples[low + 1] - samples[low], samples[low])
}

fn report(term: &str, samples: Vec<f64>) {
    println!(
        "{{\"term\":\"{term}\",\"n\":{},\"median_ms\":{:.4},\
         \"p2_5_ms\":{:.4},\"p97_5_ms\":{:.4}}}",
        samples.len(),
        median_ms(samples.clone()),
        percentile_ms(samples.clone(), 25, 1000),
        percentile_ms(samples, 975, 1000)
    );
}

fn time<T>(operation: impl Fn() -> T) -> f64 {
    let started = Instant::now();
    let result = operation();
    let elapsed = started.elapsed().as_secs_f64() * 1000.0;
    drop(result);
    elapsed
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_owned()
}

#[test]
#[ignore = "reads the live cache root and reports timings; operator-run only"]
fn warm_terms_are_reported() -> Result<(), TestError> {
    let root = cache_root()?;
    let name = subbinary();
    let version = version()?;

    let scans: Vec<f64> = (0..SAMPLES)
        .map(|_| time(|| cache::find(&root, &name, &version)))
        .collect();
    report("cache::find", scans);

    let cached = cache::find(&root, &name, &version).ok_or_else(|| {
        format!("no cached {name} {version} under {}", root.display())
    })?;
    let keys = TrustedKeys::embedded()?;

    // The replica of the resolver's private `reverify`: read the bytes, read
    // the detached signature, then `verify_binary`, which compares the
    // name-derived sha256 and only then checks the minisign signature.
    let reverifications: Vec<f64> = (0..SAMPLES)
        .map(|_| {
            time(|| -> Result<(), TestError> {
                let bytes = std::fs::read(&cached.path)?;
                let signature =
                    std::fs::read_to_string(&cached.signature_path)?;
                verifier::verify_binary(
                    &file_name(&cached.path),
                    &bytes,
                    &cached.sha256,
                    &signature,
                    &keys,
                )?;
                Ok(())
            })
        })
        .collect();
    report("reverify", reverifications);

    let bytes = std::fs::read(&cached.path)?;
    let digests: Vec<f64> = (0..SAMPLES)
        .map(|_| time(|| verifier::sha256_hex(&bytes)))
        .collect();
    report("verifier::sha256_hex", digests);

    let signature = std::fs::read_to_string(&cached.signature_path)?;
    let signatures: Vec<f64> = (0..SAMPLES)
        .map(|_| time(|| keys.verifies(&bytes, &signature)))
        .collect();
    report("TrustedKeys::verifies", signatures);

    println!("{{\"asset_bytes\":{}}}", bytes.len());
    Ok(())
}
