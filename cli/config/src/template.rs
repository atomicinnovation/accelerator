//! Resolving a template by name through its three tiers: the configured
//! path, the user templates directory, and the plugin default.

use crate::ConfigAccess;
use crate::ConfigError;
use crate::Key;
use crate::ReadTemplate;
use crate::ResolvedTemplate;

/// Resolves a template by name, or `None` when it is not found in any tier.
///
/// A non-empty `kind` attempts `<name>-<kind>` first and falls back to
/// `<name>`, so a kind-discriminated document resolves its kind-specific
/// template when one exists and the general one otherwise.
///
/// # Errors
///
/// [`ConfigError::Invalid`] when the name is not an identifier, or a
/// [`ConfigError`] when a config value or candidate file cannot be read.
pub fn resolve(
    config: &dyn ConfigAccess,
    templates: &dyn ReadTemplate,
    name: &str,
    kind: Option<&str>,
) -> Result<Option<ResolvedTemplate>, ConfigError> {
    if let Some(kind) = kind.filter(|value| !value.is_empty()) {
        let composite = format!("{name}-{kind}");
        if let Some(resolved) = resolve_one(config, templates, &composite)? {
            return Ok(Some(resolved));
        }
    }
    resolve_one(config, templates, name)
}

fn resolve_one(
    config: &dyn ConfigAccess,
    templates: &dyn ReadTemplate,
    name: &str,
) -> Result<Option<ResolvedTemplate>, ConfigError> {
    crate::validate_identifier("template name", name)?;
    let config_path = scalar(config, &format!("templates.{name}"))?;
    let dir = templates_dir(config)?;
    templates.resolve_template(name, config_path.as_deref(), &dir)
}

/// The configured user templates directory, or the built-in default.
///
/// # Errors
///
/// A [`ConfigError`] when the `paths.templates` value cannot be read.
pub fn templates_dir(config: &dyn ConfigAccess) -> Result<String, ConfigError> {
    Ok(config
        .effective_nonempty(&Key::parse("paths.templates")?, None)?
        .rendered())
}

fn scalar(
    config: &dyn ConfigAccess,
    key: &str,
) -> Result<Option<String>, ConfigError> {
    Ok(config
        .effective_nonempty(&Key::parse(key)?, None)?
        .configured_value())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::cell::RefCell;

    use crate::ConfigAccess;
    use crate::ConfigError;
    use crate::Key;
    use crate::Level;
    use crate::ReadTemplate;
    use crate::Resolved;
    use crate::ResolvedTemplate;
    use crate::TemplateSource;

    use super::resolve;

    struct AbsentConfig;

    impl ConfigAccess for AbsentConfig {
        fn get(
            &self,
            _key: &Key,
            _level: Option<Level>,
        ) -> Result<Resolved, ConfigError> {
            Ok(Resolved::Absent)
        }

        fn set(
            &self,
            _key: &Key,
            _value: &str,
            _level: Level,
        ) -> Result<(), ConfigError> {
            unreachable!("resolve never writes")
        }
    }

    struct StubTemplates {
        present: Vec<String>,
        asked: RefCell<Vec<String>>,
    }

    impl StubTemplates {
        fn new(present: &[&str]) -> Self {
            Self {
                present: present
                    .iter()
                    .map(|name| (*name).to_owned())
                    .collect(),
                asked: RefCell::new(Vec::new()),
            }
        }
    }

    impl ReadTemplate for StubTemplates {
        fn resolve_template(
            &self,
            name: &str,
            _config_path: Option<&str>,
            _templates_dir: &str,
        ) -> Result<Option<ResolvedTemplate>, ConfigError> {
            self.asked.borrow_mut().push(name.to_owned());
            Ok(self.present.iter().any(|held| held == name).then(|| {
                ResolvedTemplate {
                    source: TemplateSource::PluginDefault,
                    abs_path: format!("templates/{name}.md"),
                    display_path: format!("templates/{name}.md"),
                    content: String::new(),
                    warning: None,
                }
            }))
        }

        fn template_names(&self) -> Result<Vec<String>, ConfigError> {
            Ok(self.present.clone())
        }

        fn plugin_default(
            &self,
            _name: &str,
        ) -> Result<Option<ResolvedTemplate>, ConfigError> {
            unreachable!("resolve never asks for the plugin default")
        }
    }

    #[test]
    fn resolve_prefers_the_kind_specific_template() -> Result<(), ConfigError> {
        let templates =
            StubTemplates::new(&["topic-research-brief", "topic-research"]);
        let resolved = resolve(
            &AbsentConfig,
            &templates,
            "topic-research",
            Some("brief"),
        )?
        .expect("kind-specific template resolves");
        assert_eq!(resolved.display_path, "templates/topic-research-brief.md");
        Ok(())
    }

    #[test]
    fn resolve_falls_back_to_the_general_template() -> Result<(), ConfigError> {
        let templates = StubTemplates::new(&["topic-research"]);
        let resolved = resolve(
            &AbsentConfig,
            &templates,
            "topic-research",
            Some("report"),
        )?
        .expect("general template resolves on fallback");
        assert_eq!(resolved.display_path, "templates/topic-research.md");
        assert_eq!(
            *templates.asked.borrow(),
            vec!["topic-research-report", "topic-research"]
        );
        Ok(())
    }

    #[test]
    fn resolve_without_a_kind_makes_a_single_attempt() -> Result<(), ConfigError>
    {
        let templates = StubTemplates::new(&["plan"]);
        resolve(&AbsentConfig, &templates, "plan", None)?
            .expect("plain template resolves");
        assert_eq!(*templates.asked.borrow(), vec!["plan"]);
        Ok(())
    }

    #[test]
    fn resolve_with_an_empty_kind_makes_a_single_attempt(
    ) -> Result<(), ConfigError> {
        let templates = StubTemplates::new(&["plan"]);
        resolve(&AbsentConfig, &templates, "plan", Some(""))?
            .expect("plain template resolves");
        assert_eq!(*templates.asked.borrow(), vec!["plan"]);
        Ok(())
    }
}
