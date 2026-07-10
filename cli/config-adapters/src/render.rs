//! The value projection shared by the parity oracle and the shipped output, so
//! the two can never drift. Distinct from `document::render`, which round-trips
//! a document body.

use config::{Resolved, Scalar, Value};

/// The string a resolution miss projects to. The parity oracle passes it to the
/// bash reader as the default argument so a genuine miss compares equal on both
/// sides.
pub const ABSENT_SENTINEL: &str = "__accelerator_config_absent__";

/// Projects a resolved [`Value`] to its canonical string form.
#[must_use]
pub fn render_value(value: &Value) -> String {
    match value {
        Value::Scalar(scalar) => render_scalar(scalar),
        Value::Sequence(items) => {
            let rendered: Vec<String> =
                items.iter().map(render_scalar).collect();
            format!("[{}]", rendered.join(", "))
        }
        _ => String::new(),
    }
}

/// Projects a [`Resolved`] outcome, mapping a miss to [`ABSENT_SENTINEL`].
#[must_use]
pub fn render_resolved(resolved: &Resolved) -> String {
    match resolved {
        Resolved::Found(value) => render_value(value),
        Resolved::Absent => ABSENT_SENTINEL.to_owned(),
    }
}

fn render_scalar(scalar: &Scalar) -> String {
    match scalar {
        Scalar::String(text) => text.clone(),
        Scalar::Bool(value) => value.to_string(),
        Scalar::Int(value) => value.to_string(),
        Scalar::Float(value) => value.to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use config::{Resolved, Scalar, Value};

    use super::{render_resolved, render_value, ABSENT_SENTINEL};

    #[test]
    fn renders_scalar_kinds() {
        assert_eq!(
            render_value(&Value::Scalar(Scalar::String("x".to_owned()))),
            "x"
        );
        assert_eq!(render_value(&Value::Scalar(Scalar::Bool(true))), "true");
        assert_eq!(render_value(&Value::Scalar(Scalar::Int(42))), "42");
        assert_eq!(render_value(&Value::Scalar(Scalar::Float(1.5))), "1.5");
        assert_eq!(render_value(&Value::Scalar(Scalar::Null)), "");
    }

    #[test]
    fn renders_a_sequence_in_bracketed_form() {
        assert_eq!(
            render_value(&Value::Sequence(vec![
                Scalar::String("a".to_owned()),
                Scalar::String("b".to_owned()),
            ])),
            "[a, b]"
        );
        assert_eq!(render_value(&Value::Sequence(Vec::new())), "[]");
    }

    #[test]
    fn renders_a_miss_as_the_sentinel() {
        assert_eq!(render_resolved(&Resolved::Absent), ABSENT_SENTINEL);
    }
}
