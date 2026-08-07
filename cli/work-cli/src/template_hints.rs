//! Adapter/binary wiring for `work template-hints`: resolves the
//! `work-item` template through the three config tiers and extracts the
//! named field's hint values. Exact behavioural match for
//! `work-item-template-field-hints.sh`: always exits 0.

use config::ConfigAccess;
use config::Key;
use config::ReadTemplate;

fn templates_dir(config: &dyn ConfigAccess) -> Result<String, kernel::Error> {
    let key = Key::parse("paths.templates")
        .map_err(|error| kernel::Error::Failed(error.to_string()))?;
    Ok(config
        .effective_nonempty(&key, None)
        .map_err(|error| kernel::Error::Failed(error.to_string()))?
        .rendered())
}

fn configured_override(
    config: &dyn ConfigAccess,
    key: &str,
) -> Result<Option<String>, kernel::Error> {
    let parsed = Key::parse(key)
        .map_err(|error| kernel::Error::Failed(error.to_string()))?;
    Ok(config
        .effective_nonempty(&parsed, None)
        .map_err(|error| kernel::Error::Failed(error.to_string()))?
        .configured_value())
}

/// Prints one hint value per line for `field`, resolving the `work-item`
/// template through `templates` and falling back to
/// [`work::template_hints::hardcoded_fallback`] when the template cannot be
/// read or resolves to nothing.
pub fn run(
    config: &dyn ConfigAccess,
    templates: &dyn ReadTemplate,
    field: &str,
) {
    let hints = resolve_hints(config, templates, field);
    for hint in hints {
        println!("{hint}");
    }
}

fn resolve_hints(
    config: &dyn ConfigAccess,
    templates: &dyn ReadTemplate,
    field: &str,
) -> Vec<String> {
    let Ok(dir) = templates_dir(config) else {
        return work::template_hints::hardcoded_fallback(field);
    };
    let Ok(config_path) = configured_override(config, "templates.work-item")
    else {
        return work::template_hints::hardcoded_fallback(field);
    };
    match templates.resolve_template("work-item", config_path.as_deref(), &dir)
    {
        Ok(Some(resolved)) => {
            work::template_hints::extract_hints(&resolved.content, field)
        }
        _ => work::template_hints::hardcoded_fallback(field),
    }
}
