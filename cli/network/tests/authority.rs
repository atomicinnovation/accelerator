use network::Authority;
use network::AuthorityError;
use network::Host;
use network::HostError;

type TestError = Box<dyn std::error::Error>;

fn port_of(raw: &str) -> Result<Option<u16>, AuthorityError> {
    Authority::parse(raw).map(|authority| authority.port())
}

#[test]
fn a_port_in_range_is_read() {
    assert_eq!(port_of("example.org:8080"), Ok(Some(8080)));
    assert_eq!(port_of("[::1]:443"), Ok(Some(443)));
}

#[test]
fn an_absent_or_empty_port_leaves_the_scheme_default() {
    assert_eq!(port_of("example.org"), Ok(None));
    assert_eq!(port_of("example.org:"), Ok(None));
    assert_eq!(port_of("[::1]"), Ok(None));
}

#[test]
fn a_port_outside_one_to_65535_or_not_a_number_is_refused() {
    for raw in [
        "example.org:0",
        "example.org:99999",
        "example.org:8a",
        "example.org:+80",
        "[::1]8080",
    ] {
        assert_eq!(port_of(raw), Err(AuthorityError::Port), "{raw}");
    }
}

#[test]
fn the_host_is_canonicalised_as_host_canonicalise_does() -> Result<(), TestError>
{
    let authority = Authority::parse("[::FFFF:127.0.0.1]:8080")?;
    assert_eq!(
        authority.host(),
        &Host::canonicalise("[::FFFF:127.0.0.1]:8080")?
    );
    Ok(())
}

#[test]
fn a_host_failure_travels_out_as_a_host_error() {
    assert_eq!(
        port_of("user@example.org:80"),
        Err(AuthorityError::Host(HostError::Userinfo))
    );
    assert_eq!(
        port_of("0x7f000001:80"),
        Err(AuthorityError::Host(HostError::NumericEncoding))
    );
}

#[test]
fn host_canonicalise_still_ignores_a_malformed_port() -> Result<(), TestError> {
    assert_eq!(
        Host::canonicalise("example.org:99999")?.to_string(),
        "example.org"
    );
    Ok(())
}
