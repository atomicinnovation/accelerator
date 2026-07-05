//! Installs the pure-Rust `ring` crypto provider rustls needs, chosen over the
//! default `aws-lc-rs` (C + per-arch asm, hostile to cross-compilation).

/// Install the `ring` crypto provider as the process default (idempotent).
///
/// # Errors
///
/// Never currently; the `Result` is the seam for a fallible setup.
pub fn install_crypto_provider() -> Result<(), kernel::Error> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    Ok(())
}
