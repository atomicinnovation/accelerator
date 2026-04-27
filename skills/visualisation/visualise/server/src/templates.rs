use std::collections::HashMap;
use std::path::PathBuf;

use serde::Serialize;

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
}

pub struct TemplateResolver {
    by_name: HashMap<String, Vec<TemplateTier>>,
}

impl TemplateResolver {
    pub async fn build(
        templates: &HashMap<String, TemplateTiers>,
        driver: &dyn FileDriver,
    ) -> Self {
        let mut by_name = HashMap::new();
        for (name, tiers) in templates {
            let mut ordered = Vec::with_capacity(3);

            let config_path = tiers
                .config_override
                .clone()
                .unwrap_or_else(|| PathBuf::from(format!("<no config override for {name}>")));
            let (present, content, etag) = load_via_driver(&tiers.config_override, driver).await;
            ordered.push(TemplateTier {
                source: TemplateTierSource::ConfigOverride,
                path: config_path,
                present,
                active: false,
                content,
                etag,
            });

            let (present, content, etag) =
                load_via_driver(&Some(tiers.user_override.clone()), driver).await;
            ordered.push(TemplateTier {
                source: TemplateTierSource::UserOverride,
                path: tiers.user_override.clone(),
                present,
                active: false,
                content,
                etag,
            });

            let (present, content, etag) =
                load_via_driver(&Some(tiers.plugin_default.clone()), driver).await;
            ordered.push(TemplateTier {
                source: TemplateTierSource::PluginDefault,
                path: tiers.plugin_default.clone(),
                present,
                active: false,
                content,
                etag,
            });

            let active_source = ordered
                .iter()
                .find(|t| t.present)
                .map(|t| t.source)
                .unwrap_or(TemplateTierSource::PluginDefault);
            for t in &mut ordered {
                t.active = t.source == active_source;
            }

            by_name.insert(name.clone(), ordered);
        }
        Self { by_name }
    }

    pub fn list(&self) -> Vec<TemplateSummary> {
        let mut out: Vec<TemplateSummary> = self
            .by_name
            .iter()
            .map(|(name, tiers)| TemplateSummary {
                name: name.clone(),
                tiers: tiers
                    .iter()
                    .map(|t| TemplateTier {
                        source: t.source,
                        path: t.path.clone(),
                        present: t.present,
                        active: t.active,
                        content: None,
                        etag: t.etag.clone(),
                    })
                    .collect(),
                active_tier: tiers
                    .iter()
                    .find(|t| t.active)
                    .map(|t| t.source)
                    .unwrap_or(TemplateTierSource::PluginDefault),
            })
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    pub fn detail(&self, name: &str) -> Option<TemplateDetail> {
        let tiers = self.by_name.get(name)?;
        let active_tier = tiers
            .iter()
            .find(|t| t.active)
            .map(|t| t.source)
            .unwrap_or(TemplateTierSource::PluginDefault);
        Some(TemplateDetail {
            name: name.to_string(),
            tiers: tiers.clone(),
            active_tier,
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
        }
    }

    fn test_driver(dir: &std::path::Path) -> LocalFileDriver {
        LocalFileDriver::new(&HashMap::new(), vec![dir.to_path_buf()])
    }

    #[tokio::test]
    async fn all_three_tiers_present_picks_config_override_as_active() {
        let tmp = tempfile::tempdir().unwrap();
        let driver = test_driver(tmp.path());
        let mut map = HashMap::new();
        map.insert("adr".to_string(), tiers_all_three(tmp.path()));
        let r = TemplateResolver::build(&map, &driver).await;
        let summaries = r.list();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].active_tier, TemplateTierSource::ConfigOverride);
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
        };
        let mut map = HashMap::new();
        map.insert("adr".to_string(), t);
        let r = TemplateResolver::build(&map, &driver).await;
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
        };
        let mut map = HashMap::new();
        map.insert("adr".to_string(), t);
        let r = TemplateResolver::build(&map, &driver).await;
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
        let r = TemplateResolver::build(&map, &driver).await;
        let names: Vec<String> = r.list().into_iter().map(|s| s.name).collect();
        assert_eq!(names, vec!["adr", "plan", "research"]);
    }

    #[tokio::test]
    async fn list_omits_content_but_detail_includes_it() {
        let tmp = tempfile::tempdir().unwrap();
        let driver = test_driver(tmp.path());
        let mut map = HashMap::new();
        map.insert("adr".to_string(), tiers_all_three(tmp.path()));
        let r = TemplateResolver::build(&map, &driver).await;
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
        let r = TemplateResolver::build(&map, &driver).await;
        assert!(r.detail("missing").is_none());
    }

    #[tokio::test]
    async fn etag_is_stable_across_reads() {
        let tmp = tempfile::tempdir().unwrap();
        let driver = test_driver(tmp.path());
        let mut map = HashMap::new();
        map.insert("adr".to_string(), tiers_all_three(tmp.path()));
        let r1 = TemplateResolver::build(&map, &driver).await;
        let r2 = TemplateResolver::build(&map, &driver).await;
        let a = r1.detail("adr").unwrap();
        let b = r2.detail("adr").unwrap();
        for (ta, tb) in a.tiers.iter().zip(b.tiers.iter()) {
            assert_eq!(ta.etag, tb.etag);
        }
    }
}
