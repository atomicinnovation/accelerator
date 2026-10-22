//! Where a sync run prints: its report to one stream, its warnings to the
//! other, every warning made safe for a terminal on the way.

use std::fmt::Display;
use std::io::Write;

/// Console writes are best-effort, as `println!` is: a closed stream must
/// not turn a finished sync into a failed one.
pub struct Console<'a> {
    out: &'a mut dyn Write,
    err: &'a mut dyn Write,
}

impl<'a> Console<'a> {
    pub fn new(out: &'a mut dyn Write, err: &'a mut dyn Write) -> Self {
        Self { out, err }
    }

    pub fn report(&mut self, line: impl Display) {
        let _ = writeln!(self.out, "{line}");
    }

    pub fn warn(&mut self, message: impl Display) {
        let _ = writeln!(self.err, "{}", printable(&message.to_string()));
    }

    /// A stream for a collaborator that writes its own warnings, each line
    /// made printable as it completes.
    pub fn warnings(&mut self) -> impl Write + '_ {
        PrintableLines::new(&mut *self.err)
    }
}

/// Escapes every control character but the newline, so text from paths,
/// filenames or a tracker cannot move the cursor, clear the screen or
/// overwrite a line, while a deliberately multi-line warning keeps its
/// layout.
fn printable(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() && character != '\n' {
                character.escape_default().collect()
            } else {
                character.to_string()
            }
        })
        .collect()
}

struct PrintableLines<'a> {
    sink: &'a mut dyn Write,
    unfinished: Vec<u8>,
}

impl<'a> PrintableLines<'a> {
    fn new(sink: &'a mut dyn Write) -> Self {
        Self {
            sink,
            unfinished: Vec::new(),
        }
    }

    fn emit(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.sink
            .write_all(printable(&String::from_utf8_lossy(bytes)).as_bytes())
    }
}

impl Write for PrintableLines<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.unfinished.extend_from_slice(bytes);
        while let Some(end) =
            self.unfinished.iter().position(|&byte| byte == b'\n')
        {
            let line: Vec<u8> = self.unfinished.drain(..=end).collect();
            self.emit(&line)?;
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let rest = std::mem::take(&mut self.unfinished);
        if !rest.is_empty() {
            self.emit(&rest)?;
        }
        self.sink.flush()
    }
}

impl Drop for PrintableLines<'_> {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::io::Write as _;

    use super::Console;

    #[test]
    fn a_control_sequence_split_across_writes_is_escaped() {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        {
            let mut console = Console::new(&mut out, &mut err);
            let mut warnings = console.warnings();
            warnings.write_all(b"warning: \x1b").expect("write");
            warnings
                .write_all(b"[2J cleared\nnote: unfinished")
                .expect("write");
        }

        assert_eq!(
            String::from_utf8(err).expect("UTF-8"),
            "warning: \\u{1b}[2J cleared\nnote: unfinished"
        );
        assert!(out.is_empty());
    }

    #[test]
    fn a_multi_line_warning_keeps_its_layout() {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        Console::new(&mut out, &mut err).warn("first\n  second\tthird");

        assert_eq!(
            String::from_utf8(err).expect("UTF-8"),
            "first\n  second\\tthird\n"
        );
    }
}
