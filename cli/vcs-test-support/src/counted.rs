//! A loop that proves it ran as many times as its caller expected.

use crate::Error;

/// Runs `body` for every item, then fails unless exactly `expected` bodies
/// completed.
///
/// The caller states `expected` independently of `items`, so a loop that
/// silently runs fewer iterations, or none, fails.
///
/// # Errors
///
/// The first error `body` returns, or, when every body completes, an error
/// naming both counts if `expected` is zero or differs from the count.
pub fn each_of<T, E>(
    expected: usize,
    items: impl IntoIterator<Item = T>,
    mut body: impl FnMut(T) -> Result<(), E>,
) -> Result<(), E>
where
    E: From<Error>,
{
    let mut completed = 0;
    for item in items {
        body(item)?;
        completed += 1;
    }
    if expected == 0 || completed != expected {
        return Err(Error::message(format!(
            "expected {expected} iterations to complete, but {completed} did"
        ))
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::each_of;
    use crate::Error;

    fn completed(count: usize, expected: usize) -> Result<(), Error> {
        each_of(expected, 0..count, |_| Ok(()))
    }

    #[test]
    fn exactly_the_expected_count_passes() {
        assert!(completed(3, 3).is_ok());
    }

    #[test]
    fn fewer_than_expected_fails_naming_both_counts() {
        let error = completed(2, 3).err().map(|error| error.to_string());

        assert_eq!(
            error.as_deref(),
            Some("expected 3 iterations to complete, but 2 did")
        );
    }

    #[test]
    fn more_than_expected_fails() {
        assert!(completed(4, 3).is_err());
    }

    #[test]
    fn expecting_none_fails_even_when_none_ran() {
        assert!(completed(0, 0).is_err());
    }

    #[test]
    fn a_failing_body_fails_with_its_own_error() {
        let error = each_of(3, 0..3, |item| {
            if item == 1 {
                Err(Error::message("the second failed"))
            } else {
                Ok(())
            }
        })
        .err()
        .map(|error| error.to_string());

        assert_eq!(error.as_deref(), Some("the second failed"));
    }
}
