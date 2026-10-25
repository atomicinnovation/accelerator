use std::net::IpAddr;

use network::host_reach::classify_address;
use network::HostReach;

type TestError = Box<dyn std::error::Error>;

#[test]
fn a_resolved_address_classifies_without_a_host() -> Result<(), TestError> {
    assert_eq!(
        classify_address("169.254.169.254".parse::<IpAddr>()?),
        HostReach::LinkLocal
    );
    assert_eq!(
        classify_address("93.184.216.34".parse::<IpAddr>()?),
        HostReach::Public
    );
    Ok(())
}
