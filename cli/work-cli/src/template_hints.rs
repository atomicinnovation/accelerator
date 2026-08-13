//! Adapter/binary wiring for `work template-hints`: resolves the
//! `work-item` template through the three config tiers and extracts the
//! named field's hint values. Always exits 0.

use ::config::ConfigAccess;
use ::config::ReadTemplate;

use crate::config::configured_override;
use crate::config::templates_dir;

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
