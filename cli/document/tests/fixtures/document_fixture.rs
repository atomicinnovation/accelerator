use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        return ExitCode::FAILURE;
    };
    let Ok(content) = std::fs::read_to_string(&path) else {
        return ExitCode::FAILURE;
    };
    let _ = document::parse(&content);
    ExitCode::SUCCESS
}
