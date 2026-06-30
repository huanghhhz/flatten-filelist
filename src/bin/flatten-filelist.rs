use flatten_filelist::core;
use std::env;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let mut recursive = false;
    let mut deduplicate = false;
    let mut check_exist = false;
    let mut resolve_path = false;
    let mut encode_with_env: Option<String> = None;
    let mut filelists: Vec<String> = Vec::new();
    let mut directives: Vec<String> = Vec::new();

    let mut arg_iter = args.iter().peekable();

    // Phase 1: collect filelists (positional args at the start)
    while let Some(arg) = arg_iter.peek() {
        if arg.starts_with('-') {
            break;
        }
        filelists.push(arg_iter.next().unwrap().clone());
    }

    // Phase 2: process flags (all after filelists, order independent)
    while let Some(arg) = arg_iter.next() {
        match arg.as_str() {
            "--recursive" => recursive = true,
            "--deduplicate" => deduplicate = true,
            "--check-exist" => check_exist = true,
            "--resolve-path" => resolve_path = true,
            "--encode-with-env" => {
                match arg_iter.next() {
                    Some(name) => encode_with_env = Some(name.clone()),
                    None => {
                        eprintln!("--encode-with-env requires a value");
                        process::exit(1);
                    }
                }
            }
            "-d" | "--define" => {
                // Consume all subsequent args until next flag or end
                let before = directives.len();
                while let Some(val) = arg_iter.peek() {
                    if val.starts_with('-') {
                        break;
                    }
                    directives.push(arg_iter.next().unwrap().clone());
                }
                if directives.len() == before {
                    eprintln!("-d/--define requires at least one value");
                    process::exit(1);
                }
            }
            _ if arg.starts_with('-') && arg != "--" => {
                eprintln!("unknown flag: {}", arg);
                process::exit(1);
            }
            _ => {
                eprintln!("unexpected positional arg after flags: {}", arg);
                process::exit(1);
            }
        }
    }

    if filelists.is_empty() {
        eprintln!(
            "Usage: {} <filelist...> [--recursive] [--deduplicate] [--check-exist] [--resolve-path] [--encode-with-env <NAME>] [-d DEFINE...]",
            env::args().next().unwrap_or_else(|| "flatten-filelist".into())
        );
        process::exit(1);
    }

    let paths: Vec<&Path> = filelists.iter().map(|s| Path::new(s.as_str())).collect();

    let (content, errors) = core::read_filelists(
        &paths,
        &mut directives,
        recursive,
        deduplicate,
        check_exist,
        resolve_path,
        encode_with_env.as_deref(),
    );

    for err in &errors {
        eprintln!("ERROR: {}", err);
    }

    for line in &content {
        println!("{}", line);
    }

    if !errors.is_empty() {
        process::exit(1);
    }
}
