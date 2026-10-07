//! Migration 0008 over `FileMigrationContext` composed with `YamlFrontmatter`:
//! the YAML-dependent behaviour the domain's stub context cannot pin, and the
//! agreement between the parser port and the `document` emitter.

mod common;

use std::fs;
use std::path::Path;

use common::Composition;
use migrate::migrations::m0008::Migration0008;
use migrate::ports::MigrationContext as _;
use migrate::registry::Migration as _;
use migrate_adapters::context::FileMigrationContext;
use tempfile::TempDir;

type TestError = Box<dyn std::error::Error>;

/// A structurally-complete work item, so the re-render's own
/// `validate_file` gate has no unrelated base-field violation to trip on.
fn valid_work_item(extra_lines: &str) -> String {
    format!(
        "---\ntype: work-item\nid: \"0001\"\ntitle: Bare\n\
         date: \"2026-01-01T00:00:00+00:00\"\nauthor: Toby\ntags: []\n\
         last_updated: \"2026-01-01T00:00:00+00:00\"\n\
         last_updated_by: Toby\nschema_version: 1\nstatus: draft\n\
         kind: feature\npriority: normal\n{extra_lines}---\nbody\n"
    )
}

fn write(root: &Path, relative: &str, content: &str) -> Result<(), TestError> {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().ok_or("no parent")?)?;
    fs::write(path, content)?;
    Ok(())
}

fn migrate(root: &Path) -> Result<(), TestError> {
    let composition = Composition::at(root)?;
    let ctx = FileMigrationContext::new(root, composition.capabilities());
    Migration0008.apply(&ctx)?;
    Ok(())
}

#[test]
fn a_bare_document_is_re_rendered_with_every_string_quoted(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    write(dir.path(), "meta/work/0001-x.md", &valid_work_item(""))?;

    migrate(dir.path())?;

    let content = fs::read_to_string(dir.path().join("meta/work/0001-x.md"))?;
    assert!(content.contains("title: \"Bare\""), "{content}");
    assert!(content.contains("status: \"draft\""), "{content}");
    assert!(content.contains("schema_version: 1"), "{content}");
    assert!(content.contains("body\n"), "{content}");
    Ok(())
}

#[test]
fn a_block_linkage_sequence_with_colons_reflows_to_quoted_flow(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    write(
        dir.path(),
        "meta/work/0002-y.md",
        &valid_work_item("relates_to:\n  - work-item:0194\n  - adr:ADR-0034\n"),
    )?;

    migrate(dir.path())?;

    let content = fs::read_to_string(dir.path().join("meta/work/0002-y.md"))?;
    assert!(
        content.contains("relates_to: [\"work-item:0194\", \"adr:ADR-0034\"]"),
        "{content}"
    );
    Ok(())
}

#[test]
fn config_untyped_frontmatter_quotes_strings_and_leaves_integers_bare(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    write(
        dir.path(),
        ".accelerator/config.md",
        "---\nvisualiser:\n  port: 8080\n  theme: dark\n\
         tags:\n  - one\n  - two\n---\nbody\n",
    )?;

    migrate(dir.path())?;

    let content =
        fs::read_to_string(dir.path().join(".accelerator/config.md"))?;
    assert!(content.contains("port: 8080"), "{content}");
    assert!(content.contains("theme: \"dark\""), "{content}");
    assert!(content.contains("tags: [\"one\", \"two\"]"), "{content}");
    Ok(())
}

#[test]
fn a_value_retyping_re_render_aborts_and_writes_nothing(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let fixture = valid_work_item("ratio: 1.0\n");
    write(dir.path(), "meta/work/0004-f.md", &fixture)?;

    let Err(error) = migrate(dir.path()) else {
        return Err("float coercion must abort".into());
    };

    assert!(error.to_string().contains("0004-f.md"), "{error}");
    assert_eq!(
        fs::read_to_string(dir.path().join("meta/work/0004-f.md"))?,
        fixture
    );
    Ok(())
}

#[test]
fn a_dropped_frontmatter_comment_is_still_written() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    write(
        dir.path(),
        "meta/work/0006-c.md",
        &valid_work_item("# a standalone frontmatter comment\n"),
    )?;

    migrate(dir.path())?;

    let content = fs::read_to_string(dir.path().join("meta/work/0006-c.md"))?;
    assert!(!content.contains("standalone"), "comment must be dropped");
    assert!(content.contains("title: \"Bare\""), "{content}");
    Ok(())
}

#[test]
fn re_rendering_is_a_byte_level_fixed_point() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let composition = Composition::at(dir.path())?;
    let ctx = FileMigrationContext::new(dir.path(), composition.capabilities());
    let original = "---\ntype: work-item\nid: \"0003\"\n\
         title: A long title that runs well past eighty columns to prove \
         no block scalar refold happens here at all\n\
         tags: [alpha, beta]\nschema_version: 1\n---\nbody\n";

    let once = ctx.render_canonical(original)?;
    let twice = ctx.render_canonical(&once)?;

    assert_eq!(once, twice, "second pass must be byte-identical");
    Ok(())
}

#[test]
fn the_parser_and_the_emitter_agree_on_every_root_class(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let composition = Composition::at(dir.path())?;
    let ctx = FileMigrationContext::new(dir.path(), composition.capabilities());

    for original in [
        "no frontmatter\n",
        "---\n---\nbody\n",
        "---\n~\n---\nbody\n",
        "---\ntitle: T\ncount: 3\n---\nbody\n",
        "---\n- a\n- b\n---\nbody\n",
        "---\nscalar\n---\nbody\n",
    ] {
        let rendered = ctx.render_canonical(original)?;
        assert_eq!(
            ctx.parse_frontmatter(original)?,
            ctx.parse_frontmatter(&rendered)?,
            "{original:?} re-rendered as {rendered:?}"
        );
    }
    Ok(())
}

#[test]
fn an_unterminated_fence_fails_every_route_with_the_document_text(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let composition = Composition::at(dir.path())?;
    let ctx = FileMigrationContext::new(dir.path(), composition.capabilities());
    let unterminated = "---\ntitle: T\n";

    for error in [
        ctx.parse_frontmatter(unterminated).err(),
        ctx.frontmatter_text(unterminated).err(),
        ctx.render_canonical(unterminated).err(),
    ] {
        assert_eq!(
            error.map(|error| error.to_string()).as_deref(),
            Some("unterminated frontmatter block")
        );
    }
    Ok(())
}
