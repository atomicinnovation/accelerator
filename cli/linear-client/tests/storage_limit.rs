//! What Linear cannot store as text, read from a line of local Markdown: an
//! escaped `<` or `&` reaches Linear decoded, and an escaped `|` in a table
//! cell splits the cell, whichever escape the line uses.

use linear_client::storage_limit::StorageLimit;

fn limits(sent: &str) -> Vec<StorageLimit> {
    StorageLimit::in_sent(sent)
}

#[test]
fn an_escaped_angle_bracket_cannot_be_stored() {
    for sent in ["x &lt;b&gt; y", "x \\<b> y", "x &#60;b y", "x &#x3C;b y"] {
        assert_eq!(limits(sent), [StorageLimit::AngleBracket], "{sent}");
    }
}

#[test]
fn an_escaped_ampersand_cannot_be_stored() {
    for sent in ["x &amp;lt; y", "x \\&lt; y", "x &#38; y"] {
        assert_eq!(limits(sent), [StorageLimit::Ampersand], "{sent}");
    }
}

#[test]
fn an_escaped_pipe_in_a_table_row_cannot_be_stored() {
    for sent in ["| x \\| y | z |", "| x &#124; y | z |"] {
        assert_eq!(limits(sent), [StorageLimit::TableCellPipe], "{sent}");
    }
}

#[test]
fn an_escaped_pipe_outside_a_table_is_stored() {
    assert_eq!(limits("x \\| y"), []);
}

#[test]
fn raw_markup_and_plain_text_are_stored_as_sent() {
    for sent in ["x <b>bold</b> y", "fish & chips", "See crates.io."] {
        assert_eq!(limits(sent), [], "{sent}");
    }
}

#[test]
fn a_line_names_each_limit_it_meets_once() {
    assert_eq!(
        limits("| &lt;a&gt; \\| &amp; \\< |"),
        [
            StorageLimit::AngleBracket,
            StorageLimit::Ampersand,
            StorageLimit::TableCellPipe
        ]
    );
}

#[test]
fn each_limit_names_the_character_lost() {
    assert_eq!(StorageLimit::AngleBracket.character(), '<');
    assert_eq!(StorageLimit::Ampersand.character(), '&');
    assert_eq!(StorageLimit::TableCellPipe.character(), '|');
}
