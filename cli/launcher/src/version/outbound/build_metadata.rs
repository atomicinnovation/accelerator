//! The production [`BuildMetadata`] adapter — serves the facts vergen baked
//! into the binary (via `build.rs`) as `cargo:rustc-env` vars.

use crate::version::core::BuildMetadata;

/// Reads the build-baked metadata. Git/build facts use `option_env!` so a build
/// that could not resolve them degrades to `"unknown"` instead of failing to
/// compile.
pub struct VergenBuildMetadata;

fn or_unknown(value: Option<&'static str>) -> &'static str {
    value.unwrap_or("unknown")
}

impl BuildMetadata for VergenBuildMetadata {
    fn crate_version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    fn commit_sha(&self) -> &'static str {
        or_unknown(option_env!("VERGEN_GIT_SHA"))
    }

    fn build_date(&self) -> &'static str {
        or_unknown(option_env!("VERGEN_BUILD_TIMESTAMP"))
    }

    fn target_triple(&self) -> &'static str {
        or_unknown(option_env!("VERGEN_CARGO_TARGET_TRIPLE"))
    }
}

#[cfg(test)]
mod tests {
    use super::{or_unknown, VergenBuildMetadata};
    use crate::version::core::BuildMetadata;

    #[test]
    fn a_missing_fact_degrades_to_unknown() {
        assert_eq!(or_unknown(None), "unknown");
    }

    #[test]
    fn a_present_fact_is_passed_through() {
        assert_eq!(or_unknown(Some("abc123")), "abc123");
    }

    #[test]
    fn crate_version_is_the_launcher_cargo_version() {
        assert_eq!(
            VergenBuildMetadata.crate_version(),
            env!("CARGO_PKG_VERSION")
        );
    }

    #[test]
    fn commit_sha_reads_its_vergen_key_or_unknown() {
        assert_eq!(
            VergenBuildMetadata.commit_sha(),
            option_env!("VERGEN_GIT_SHA").unwrap_or("unknown")
        );
    }

    #[test]
    fn build_date_reads_its_vergen_key_or_unknown() {
        assert_eq!(
            VergenBuildMetadata.build_date(),
            option_env!("VERGEN_BUILD_TIMESTAMP").unwrap_or("unknown")
        );
    }

    #[test]
    fn target_triple_reads_its_vergen_key_or_unknown() {
        assert_eq!(
            VergenBuildMetadata.target_triple(),
            option_env!("VERGEN_CARGO_TARGET_TRIPLE").unwrap_or("unknown")
        );
    }
}
