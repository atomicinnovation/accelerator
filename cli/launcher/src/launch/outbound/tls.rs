//! Installs the pure-Rust `ring` crypto provider rustls needs, chosen over the
//! default `aws-lc-rs` (C + per-arch asm, hostile to cross-compilation), and
//! names the roots every TLS client trusts.

/// Install the `ring` crypto provider as the process default (idempotent).
///
/// # Errors
///
/// Never currently; the `Result` is the seam for a fallible setup.
pub fn install_crypto_provider() -> Result<(), kernel::Error> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    Ok(())
}

/// The Mozilla roots webpki-root-certs bundles, so a client trusts the same
/// roots on every host rather than whatever certificate store it finds there.
pub fn bundled_trust_roots() -> impl Iterator<Item = reqwest::Certificate> {
    webpki_root_certs::TLS_SERVER_ROOT_CERTS
        .iter()
        .filter_map(|root| reqwest::Certificate::from_der(root).ok())
}
