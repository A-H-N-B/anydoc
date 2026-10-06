//! Convert every supported document in a folder to Markdown:
//! `cargo run --release --example convert_dir -- <input dir> [-o out dir]`
//!
//! Walks the input folder recursively and writes `<name>.md` next to each
//! document, or under the output folder with the same relative layout. Files
//! whose format is not recognized are skipped; a failed conversion is reported
//! and the run continues.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anydoc::Format;

const USAGE: &str = "usage: convert_dir <input dir> [-o out dir]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut input: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                output = args.get(i).map(PathBuf::from);
            }
            other => input = Some(PathBuf::from(other)),
        }
        i += 1;
    }
    let Some(input) = input.filter(|p| p.is_dir()) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };

    let mut files = Vec::new();
    collect(&input, &mut files);
    files.sort();

    let (mut converted, mut failed) = (0, 0);
    for file in &files {
        // Markdown is the output format, so never feed earlier results back in.
        if file.extension().is_some_and(|e| e.eq_ignore_ascii_case("md")) {
            continue;
        }
        let Ok(bytes) = std::fs::read(file) else { continue };
        let Some(format) = Format::from_bytes(&bytes).or_else(|| Format::from_path(file)) else {
            continue;
        };
        let relative = file.strip_prefix(&input).unwrap_or(file);
        let target = output.as_deref().unwrap_or(&input).join(relative).with_extension("md");
        let result = anydoc::to_markdown_bytes(&bytes, format).map_err(|e| e.to_string()).and_then(
            |markdown| {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                std::fs::write(&target, markdown).map_err(|e| e.to_string())
            },
        );
        match result {
            Ok(()) => {
                converted += 1;
                eprintln!("ok    {} -> {}", file.display(), target.display());
            }
            Err(e) => {
                failed += 1;
                eprintln!("fail  {}: {e}", file.display());
            }
        }
    }
    eprintln!("{converted} converted, {failed} failed");
    if failed == 0 { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

fn collect(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, files);
        } else {
            files.push(path);
        }
    }
}
