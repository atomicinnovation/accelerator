use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::config::TemplateTiers;
use crate::file_driver::FileDriver;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TemplateTierSource {
    ConfigOverride,
    UserOverride,
    PluginDefault,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateTier {
    pub source: TemplateTierSource,
    pub path: PathBuf,
    pub present: bool,
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    /// Project-root-relative path of the config file in which the
    /// `config_override` entry for this template is declared (e.g.
    /// `.accelerator/config.md` or `.accelerator/config.local.md`).
    /// Only meaningful for the `ConfigOverride` tier; `None` for the
    /// user-override and plugin-default tiers, and for config-override
    /// tiers whose source file is unknown to the launcher.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_source: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSummary {
    pub name: String,
    pub tiers: Vec<TemplateTier>,
    pub active_tier: TemplateTierSource,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateDetail {
    pub name: String,
    pub tiers: Vec<TemplateTier>,
    pub active_tier: TemplateTierSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

struct TemplateEntry {
    tiers: Vec<TemplateTier>,
    active_tier: TemplateTierSource,
    sha256: Option<String>,
}

pub struct TemplateResolver {
    by_name: HashMap<String, TemplateEntry>,
}

/// Convert an absolute path into the display form used by the
/// templates view. The strip-order depends on the tier:
///
/// - **Plugin-default tier:** always prefer the `plugin_root` strip
///   first. The plugin default lives semantically under the plugin
///   root, and rendering it with a synthetic `<plugin-root>` prefix is
///   load-bearing for the tier-3 card and the preview-pane header.
///   This matters when the plugin happens to live inside the project
///   root (e.g. during local development of the plugin itself), where
///   a naive project-root strip would otherwise hide the prefix.
///
/// - **Other tiers:** prefer the `project_root` strip first (so
///   `<project>/.accelerator/templates/adr.md` becomes
///   `.accelerator/templates/adr.md`), falling through to the
///   `<plugin-root>` form for paths that sit outside the project.
///
/// - **Neither match:** the absolute path is passed through verbatim.
fn display_path(
    path: &Path,
    project_root: &Path,
    plugin_root: &Path,
    tier: TemplateTierSource,
) -> PathBuf {
    let strip_plugin = |p: &Path| {
        p.strip_prefix(plugin_root)
            .ok()
            .map(|rel| PathBuf::from("<plugin-root>").join(rel))
    };
    let strip_project =
        |p: &Path| p.strip_prefix(project_root).ok().map(Path::to_path_buf);
    let (first, second) = match tier {
        TemplateTierSource::PluginDefault => {
            (strip_plugin(path), strip_project(path))
        }
        _ => (strip_project(path), strip_plugin(path)),
    };
    first.or(second).unwrap_or_else(|| path.to_path_buf())
}

/// `sha256-<64-hex>` form, matching the per-tier etag shape.
/// `None` for empty content (empty-string digest is suppressed so an
/// empty winning file produces no displayable hash) and for absent
/// content.
pub(crate) fn content_sha256(content: Option<&str>) -> Option<String> {
    content.filter(|s| !s.is_empty()).map(|s| {
        format!("sha256-{}", hex::encode(Sha256::digest(s.as_bytes())))
    })
}

impl TemplateResolver {
    pub async fn build(
        templates: &HashMap<String, TemplateTiers>,
        driver: &dyn FileDriver,
        project_root: &Path,
        plugin_root: &Path,
    ) -> Self {
        let mut by_name = HashMap::new();
        for (name, tiers) in templates {
            let mut ordered = Vec::with_capacity(3);

            let config_path =
                tiers.config_override.clone().unwrap_or_else(|| {
                    PathBuf::from(format!("<no config override for {name}>"))
                });
            let (present, content, etag) =
                load_via_driver(&tiers.config_override, driver).await;
            ordered.push(TemplateTier {
                source: TemplateTierSource::ConfigOverride,
                path: display_path(
                    &config_path,
                    project_root,
                    plugin_root,
                    TemplateTierSource::ConfigOverride,
                ),
                present,
                active: false,
                content,
                etag,
                config_source: tiers.config_override_source.clone(),
            });

            let (present, content, etag) =
                load_via_driver(&Some(tiers.user_override.clone()), driver)
                    .await;
            ordered.push(TemplateTier {
                source: TemplateTierSource::UserOverride,
                path: display_path(
                    &tiers.user_override,
                    project_root,
                    plugin_root,
                    TemplateTierSource::UserOverride,
                ),
                present,
                active: false,
                content,
                etag,
                config_source: None,
            });

            let (present, content, etag) =
                load_via_driver(&Some(tiers.plugin_default.clone()), driver)
                    .await;
            ordered.push(TemplateTier {
                source: TemplateTierSource::PluginDefault,
                path: display_path(
                    &tiers.plugin_default,
                    project_root,
                    plugin_root,
                    TemplateTierSource::PluginDefault,
                ),
                present,
                active: false,
                content,
                etag,
                config_source: None,
            });

            let active_source = ordered
                .iter()
                .find(|t| t.present)
                .map_or(TemplateTierSource::PluginDefault, |t| t.source);
            for t in &mut ordered {
                t.active = t.source == active_source;
            }

            let sha256 = content_sha256(
                ordered
                    .iter()
                    .find(|t| t.active && t.present)
                    .and_then(|t| t.content.as_deref()),
            );

            by_name.insert(
                name.clone(),
                TemplateEntry {
                    tiers: ordered,
                    active_tier: active_source,
                    sha256,
                },
            );
        }
        Self { by_name }
    }

    pub fn list(&self) -> Vec<TemplateSummary> {
        let mut out: Vec<TemplateSummary> = self
            .by_name
            .iter()
            .map(|(name, entry)| TemplateSummary {
                name: name.clone(),
                tiers: entry
                    .tiers
                    .iter()
                    .map(|t| TemplateTier {
                        source: t.source,
                        path: t.path.clone(),
                        present: t.present,
                        active: t.active,
                        content: None,
                        etag: t.etag.clone(),
                        config_source: t.config_source.clone(),
                    })
                    .collect(),
                active_tier: entry.active_tier,
            })
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    pub fn detail(&self, name: &str) -> Option<TemplateDetail> {
        let entry = self.by_name.get(name)?;
        Some(TemplateDetail {
            name: name.to_string(),
            tiers: entry.tiers.clone(),
            active_tier: entry.active_tier,
            sha256: entry.sha256.clone(),
        })
    }

    pub fn names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.by_name.keys().cloned().collect();
        v.sort();
        v
    }
}

async fn load_via_driver(
    path: &Option<PathBuf>,
    driver: &dyn FileDriver,
) -> (bool, Option<String>, Option<String>) {
    let Some(p) = path else {
        return (false, None, None);
    };
    match driver.read(p).await {
        Ok(fc) => {
            let content = String::from_utf8(fc.bytes).ok();
            (true, content, Some(fc.etag))
        }
        Err(_) => (false, None, None),
    }
}

#[cfg(test)]
mod display_path_tests {
    use super::*;

    #[test]
    fn project_root_strip_wins_for_user_override_tier() {
        let project = std::path::Path::new("/work/repo");
        let plugin = std::path::Path::new("/elsewhere/plugin");
        let path =
            std::path::Path::new("/work/repo/.accelerator/templates/adr.md");
        let got = display_path(
            path,
            project,
            plugin,
            TemplateTierSource::UserOverride,
        );
        assert_eq!(got, std::path::Path::new(".accelerator/templates/adr.md"));
    }

    #[test]
    fn plugin_root_prefix_wins_for_plugin_default_tier_even_when_path_is_under_project_root(
    ) {
        // Local plugin-dev scenario: the plugin lives inside the project
        // root. Naive ordering would strip project_root first and lose the
        // `<plugin-root>` prefix; the tier-aware logic must keep it.
        let project = std::path::Path::new("/work/repo");
        let plugin = std::path::Path::new("/work/repo");
        let path = std::path::Path::new("/work/repo/templates/adr.md");
        let got = display_path(
            path,
            project,
            plugin,
            TemplateTierSource::PluginDefault,
        );
        assert_eq!(got, std::path::Path::new("<plugin-root>/templates/adr.md"));
    }

    #[test]
    fn plugin_root_prefix_used_for_plugin_default_outside_project_root() {
        let project = std::path::Path::new("/work/repo");
        let plugin = std::path::Path::new("/elsewhere/plugin");
        let path = std::path::Path::new("/elsewhere/plugin/templates/adr.md");
        let got = display_path(
            path,
            project,
            plugin,
            TemplateTierSource::PluginDefault,
        );
        assert_eq!(got, std::path::Path::new("<plugin-root>/templates/adr.md"));
    }

    #[test]
    fn plugin_default_falls_back_to_project_root_strip_when_outside_plugin_root(
    ) {
        let project = std::path::Path::new("/work/repo");
        let plugin = std::path::Path::new("/elsewhere/plugin");
        let path = std::path::Path::new("/work/repo/local/templates/adr.md");
        let got = display_path(
            path,
            project,
            plugin,
            TemplateTierSource::PluginDefault,
        );
        // Plugin-root strip fails → falls through to project-root strip.
        assert_eq!(got, std::path::Path::new("local/templates/adr.md"));
    }

    #[test]
    fn path_outside_both_roots_passes_through_verbatim() {
        let project = std::path::Path::new("/work/repo");
        let plugin = std::path::Path::new("/elsewhere/plugin");
        let path = std::path::Path::new("/somewhere/else/templates/adr.md");
        let got = display_path(
            path,
            project,
            plugin,
            TemplateTierSource::UserOverride,
        );
        assert_eq!(got, path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_driver::LocalFileDriver;

    fn tier(dir: &std::path::Path, name: &str, content: &str) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, content).unwrap();
        p
    }

    fn tiers_all_three(dir: &std::path::Path) -> TemplateTiers {
        TemplateTiers {
            config_override: Some(tier(dir, "cfg-adr.md", "from config")),
            user_override: tier(dir, "user-adr.md", "from user"),
            plugin_default: tier(dir, "plugin-adr.md", "from plugin"),
            config_override_source: None,
        }
    }

    fn test_driver(dir: &std::path::Path) -> LocalFileDriver {
        LocalFileDriver::new(&HashMap::new(), vec![dir.to_path_buf()], vec![])
    }

    #[tokio::test]
    async fn all_three_tiers_present_picks_config_override_as_active() {
        let tmp = tempfile::tempdir().unwrap();
        let driver = test_driver(tmp.path());
        let mut map = HashMap::new();
        map.insert("adr".to_string(), tiers_all_three(tmp.path()));
        let r = TemplateResolver::build(&map, &driver, tmp.path(), tmp.path())
            .await;
        let summaries = r.list();
        assert_eq!(summaries.len(), 1);
        assert_eq!(
            summaries[0].active_tier,
            TemplateTierSource::ConfigOverride
        );
        let active = summaries[0].tiers.iter().find(|t| t.active).unwrap();
        assert_eq!(active.source, TemplateTierSource::ConfigOverride);
    }

    #[tokio::test]
    async fn only_plugin_default_present_picks_plugin_default_active() {
        let tmp = tempfile::tempdir().unwrap();
        let driver = test_driver(tmp.path());
        let t = TemplateTiers {
            config_override: None,
            user_override: tmp.path().join("missing-user.md"),
            plugin_default: tier(tmp.path(), "plugin-adr.md", "from plugin"),
            config_override_source: None,
        };
        let mut map = HashMap::new();
        map.insert("adr".to_string(), t);
        let r = TemplateResolver::build(&map, &driver, tmp.path(), tmp.path())
            .await;
        let d = r.detail("adr").unwrap();
        assert_eq!(d.active_tier, TemplateTierSource::PluginDefault);
        assert_eq!(
            d.tiers.iter().filter(|t| t.present).count(),
            1,
            "only plugin-default should be present",
        );
    }

    #[tokio::test]
    async fn user_override_wins_when_config_override_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let driver = test_driver(tmp.path());
        let t = TemplateTiers {
            config_override: None,
            user_override: tier(tmp.path(), "user-adr.md", "from user"),
            plugin_default: tier(tmp.path(), "plugin-adr.md", "from plugin"),
            config_override_source: None,
        };
        let mut map = HashMap::new();
        map.insert("adr".to_string(), t);
        let r = TemplateResolver::build(&map, &driver, tmp.path(), tmp.path())
            .await;
        let d = r.detail("adr").unwrap();
        assert_eq!(d.active_tier, TemplateTierSource::UserOverride);
    }

    #[tokio::test]
    async fn list_sorts_names_alphabetically() {
        let tmp = tempfile::tempdir().unwrap();
        let driver = test_driver(tmp.path());
        let mut map = HashMap::new();
        map.insert("plan".to_string(), tiers_all_three(tmp.path()));
        map.insert("adr".to_string(), tiers_all_three(tmp.path()));
        map.insert("research".to_string(), tiers_all_three(tmp.path()));
        let r = TemplateResolver::build(&map, &driver, tmp.path(), tmp.path())
            .await;
        let names: Vec<String> = r.list().into_iter().map(|s| s.name).collect();
        assert_eq!(names, vec!["adr", "plan", "research"]);
    }

    #[tokio::test]
    async fn list_omits_content_but_detail_includes_it() {
        let tmp = tempfile::tempdir().unwrap();
        let driver = test_driver(tmp.path());
        let mut map = HashMap::new();
        map.insert("adr".to_string(), tiers_all_three(tmp.path()));
        let r = TemplateResolver::build(&map, &driver, tmp.path(), tmp.path())
            .await;
        let s = &r.list()[0];
        assert!(s.tiers.iter().all(|t| t.content.is_none()));
        let d = r.detail("adr").unwrap();
        let present_with_content = d
            .tiers
            .iter()
            .filter(|t| t.present && t.content.is_some())
            .count();
        assert_eq!(present_with_content, 3);
    }

    #[tokio::test]
    async fn detail_of_unknown_name_is_none() {
        let tmp = tempfile::tempdir().unwrap();
        let driver = test_driver(tmp.path());
        let mut map = HashMap::new();
        map.insert("adr".to_string(), tiers_all_three(tmp.path()));
        let r = TemplateResolver::build(&map, &driver, tmp.path(), tmp.path())
            .await;
        assert!(r.detail("missing").is_none());
    }

    #[tokio::test]
    async fn detail_sha256_matches_winning_tier_content_prefixed() {
        let tmp = tempfile::tempdir().unwrap();
        let driver = test_driver(tmp.path());
        let mut map = HashMap::new();
        map.insert("adr".to_string(), tiers_all_three(tmp.path()));
        let r = TemplateResolver::build(&map, &driver, tmp.path(), tmp.path())
            .await;
        let d = r.detail("adr").unwrap();
        let active = d.tiers.iter().find(|t| t.active).unwrap();
        let expected = format!(
            "sha256-{}",
            hex::encode(sha2::Sha256::digest(
                active.content.as_ref().unwrap().as_bytes(),
            )),
        );
        assert_eq!(d.sha256.as_deref(), Some(expected.as_str()));
    }

    #[tokio::test]
    async fn detail_sha256_omitted_when_winning_content_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let empty_plugin = tier(tmp.path(), "plugin-empty.md", "");
        let t = TemplateTiers {
            config_override: None,
            user_override: tmp.path().join("missing-user.md"),
            plugin_default: empty_plugin,
            config_override_source: None,
        };
        let driver = test_driver(tmp.path());
        let mut map = HashMap::new();
        map.insert("e".to_string(), t);
        let r = TemplateResolver::build(&map, &driver, tmp.path(), tmp.path())
            .await;
        let d = r.detail("e").unwrap();
        assert!(d.sha256.is_none(), "empty winning content -> no sha256");
    }

    #[tokio::test]
    async fn detail_sha256_cached_at_build_time() {
        let tmp = tempfile::tempdir().unwrap();
        let driver = test_driver(tmp.path());
        let mut map = HashMap::new();
        map.insert("adr".to_string(), tiers_all_three(tmp.path()));
        let r = TemplateResolver::build(&map, &driver, tmp.path(), tmp.path())
            .await;
        let s1 = r.detail("adr").unwrap().sha256.clone();
        let s2 = r.detail("adr").unwrap().sha256.clone();
        assert_eq!(s1, s2);
        assert!(s1.is_some());
    }

    #[tokio::test]
    async fn etag_is_stable_across_reads() {
        let tmp = tempfile::tempdir().unwrap();
        let driver = test_driver(tmp.path());
        let mut map = HashMap::new();
        map.insert("adr".to_string(), tiers_all_three(tmp.path()));
        let r1 = TemplateResolver::build(&map, &driver, tmp.path(), tmp.path())
            .await;
        let r2 = TemplateResolver::build(&map, &driver, tmp.path(), tmp.path())
            .await;
        let a = r1.detail("adr").unwrap();
        let b = r2.detail("adr").unwrap();
        for (ta, tb) in a.tiers.iter().zip(b.tiers.iter()) {
            assert_eq!(ta.etag, tb.etag);
        }
    }
}
