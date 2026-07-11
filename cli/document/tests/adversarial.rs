use std::fmt::Write;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn deeply_nested() -> String {
    let depth = 5000;
    let mut input = String::from("---\nkey: ");
    for _ in 0..depth {
        input.push('[');
    }
    for _ in 0..depth {
        input.push(']');
    }
    input.push_str("\n---\n");
    input
}

fn alias_bomb() -> String {
    let mut input = String::from("---\na0: &a0 [x, x]\n");
    for level in 1..30 {
        let _ = writeln!(
            input,
            "a{level}: &a{level} [*a{prev}, *a{prev}]",
            prev = level - 1
        );
    }
    input.push_str("---\n");
    input
}

fn trailing_whitespace_quoted_scalar() -> String {
    let title = format!("Security Lens{}", " ".repeat(34));
    format!("---\ntitle: \"{title}\"\nstatus: done\n---\nbody\n")
}

const STRUCTURALLY_MALFORMED: &str = "---\nkey: : :\n  - broken\n---\nbody\n";

const BOUND: Duration = Duration::from_secs(10);

fn parse_ok_within(input: &str) -> Option<bool> {
    let (sender, receiver) = mpsc::channel();
    let owned = input.to_owned();
    thread::spawn(move || {
        let ok = document::parse(&owned).is_ok();
        let _ = sender.send(ok);
    });
    receiver.recv_timeout(BOUND).ok()
}

#[test]
fn hostile_inputs_are_rejected_within_the_bound() {
    let cases = [
        ("deeply-nested", deeply_nested()),
        ("alias-bomb", alias_bomb()),
        ("structurally-malformed", STRUCTURALLY_MALFORMED.to_owned()),
    ];
    for (name, input) in cases {
        assert_eq!(
            parse_ok_within(&input),
            Some(false),
            "{name}: expected a bounded rejection, not a hang/panic/parse"
        );
    }
}

#[test]
fn trailing_whitespace_quoted_scalar_parses_cleanly_within_the_bound() {
    let input = trailing_whitespace_quoted_scalar();
    assert_eq!(
        parse_ok_within(&input),
        Some(true),
        "valid YAML that once crashed libyml must now parse cleanly"
    );
}
