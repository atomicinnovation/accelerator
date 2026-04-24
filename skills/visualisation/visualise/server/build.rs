// This value duplicates `accelerator_visualiser::assets::FRONTEND_DIST_REL`.
// `build.rs` runs before the crate compiles, so we cannot import from it.
// Keep the two literals in sync — tests verify the dist path resolves to a
// real directory under the manifest root.
const FRONTEND_DIST_REL: &str = "../frontend/dist";

fn main() {
    let is_embed = std::env::var("CARGO_FEATURE_EMBED_DIST").is_ok();
    if is_embed {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let dist_dir = std::path::Path::new(&manifest).join(FRONTEND_DIST_REL);
        let dist_index = dist_dir.join("index.html");
        if !dist_index.exists() {
            panic!(
                "frontend/dist/index.html not found — \
                 run `npm run build` in skills/visualisation/visualise/frontend/ \
                 before `cargo build` (or use `--features dev-frontend` to \
                 skip embedding and serve from disk instead)"
            );
        }
        println!("cargo:rerun-if-changed={FRONTEND_DIST_REL}");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
