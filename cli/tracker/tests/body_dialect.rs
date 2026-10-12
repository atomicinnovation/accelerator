//! The body dialect's identity defaults, which a tracker storing Markdown
//! verbatim relies on.

use std::borrow::Cow;

use tracker::BodyDialect;
use tracker::PlainDialect;

struct DefaultsOnly;

impl BodyDialect for DefaultsOnly {
    fn is_wire_document(&self, _body: &str) -> bool {
        true
    }
}

#[test]
fn the_default_local_body_is_the_description_verbatim() {
    let description = "\n# Heading\n\n- item  \n";

    assert_eq!(
        DefaultsOnly.local_body(description),
        Ok(description.to_owned())
    );
}

#[test]
fn the_default_as_pushed_body_is_the_body_verbatim() {
    let body = "* item\n\n\n";

    assert!(matches!(
        DefaultsOnly.as_pushed(body),
        Cow::Borrowed(pushed) if pushed == body
    ));
}

#[test]
fn the_plain_dialect_is_the_identity() {
    let body = "{\"type\":\"doc\",\"version\":1,\"content\":[]}";

    assert_eq!(PlainDialect.local_body(body), Ok(body.to_owned()));
    assert_eq!(PlainDialect.as_pushed(body), body);
    assert!(!PlainDialect.is_wire_document(body));
}
