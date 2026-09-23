//! The canonical string projection of a resolved value, shared by the shipped
//! scalar output and the parity oracle so the two can never drift.

use crate::node::Scalar;
use crate::service::Value;

/// Projects a resolved [`Value`] to its canonical string form.
///
/// A scalar renders to its bare text, a sequence to the bracketed `[a, b]`
/// form, and a mapping to the empty string — a block has no single-line scalar
/// rendering, and `dump` renders it through a separate row-group path.
#[must_use]
pub fn render_value(value: &Value) -> String {
    match value {
        Value::Scalar(scalar) => render_scalar(scalar),
        Value::Sequence(items) => {
            let rendered: Vec<String> =
                items.iter().map(render_scalar).collect();
            format!("[{}]", rendered.join(", "))
        }
        Value::Mapping(_) => String::new(),
    }
}

pub(crate) fn render_scalar(scalar: &Scalar) -> String {
    match scalar {
        Scalar::String(text) => text.clone(),
        Scalar::Bool(value) => value.to_string(),
        Scalar::Int(value) => value.to_string(),
        Scalar::Float(value) => value.to_string(),
        Scalar::Null => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::render_value;
    use crate::node::Scalar;
    use crate::service::Value;

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
    fn renders_a_mapping_as_the_empty_string() {
        assert_eq!(
            render_value(&Value::Mapping(vec![(
                "label".to_owned(),
                Value::Scalar(Scalar::String("bug".to_owned())),
            )])),
            ""
        );
        assert_eq!(render_value(&Value::Mapping(Vec::new())), "");
    }
}
