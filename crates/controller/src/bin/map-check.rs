//! CLI: validate mapping bundles without hardware.

use std::path::PathBuf;
use std::process::ExitCode;

use controller::check::{check_all_mappings, check_bundle_dir};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: map-check <bundle-dir>");
        eprintln!("       map-check --all <mappings-root>");
        return ExitCode::from(2);
    }

    if args[0] == "--all" {
        let root = args
            .get(1)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("../mappings"));
        match check_all_mappings(&root) {
            Ok(names) => {
                for n in &names {
                    println!("ok  {n}");
                }
                println!("{} bundle(s) ok", names.len());
                ExitCode::SUCCESS
            }
            Err(errs) => {
                for (name, e) in &errs {
                    eprintln!("ERR {name}: {e}");
                }
                ExitCode::FAILURE
            }
        }
    } else {
        let dir = PathBuf::from(&args[0]);
        // Allow `map-check <id>` relative to mappings/ when not a path.
        let path = if dir.is_dir() {
            dir
        } else {
            let under = PathBuf::from("../mappings").join(&args[0]);
            if under.is_dir() {
                under
            } else {
                PathBuf::from("mappings").join(&args[0])
            }
        };
        match check_bundle_dir(&path) {
            Ok(()) => {
                println!("ok  {}", path.display());
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("ERR {}: {e}", path.display());
                ExitCode::FAILURE
            }
        }
    }
}
