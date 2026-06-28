use flatten_filelist::core;
use std::env;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let mut no_recursive = false;
    let mut deduplication = false;
    let mut check_exist = false;
    let mut resolve_path = false;
    let mut encode_with_env: Option<String> = None;
    let mut filelists: Vec<String> = Vec::new();
    let mut directives: Vec<String> = Vec::new();
    let mut collecting_directives = false;

    let mut arg_iter = args.iter().peekable();

    while let Some(arg) = arg_iter.next() {
        match arg.as_str() {
            "--no-recursive" if !collecting_directives => no_recursive = true,
            "--deduplication" if !collecting_directives => deduplication = true,
            "--check-exist" if !collecting_directives => check_exist = true,
            "--resolve-path" if !collecting_directives => resolve_path = true,
            "--encode-with-env" if !collecting_directives => {
                match arg_iter.next() {
                    Some(name) => encode_with_env = Some(name.clone()),
                    None => {
                        eprintln!("--encode-with-env requires a value");
                        process::exit(1);
                    }
                }
            }
            "-d" | "--define" => collecting_directives = true,
            _ if arg.starts_with('-') && arg != "--" => {
                eprintln!("unknown flag: {}", arg);
                process::exit(1);
            }
            _ => {
                if collecting_directives {
                    directives.push(arg.clone());
                } else {
                    filelists.push(arg.clone());
                }
            }
        }
    }

    if filelists.is_empty() {
        eprintln!(
            "Usage: {} [--no-recursive] [--deduplication] [--check-exist] [--resolve-path] [--encode-with-env <NAME>] <filelist...> [-d DEFINE...]",
            env::args().next().unwrap_or_else(|| "flatten-filelist".into())
        );
        process::exit(1);
    }

    let paths: Vec<&Path> = filelists.iter().map(|s| Path::new(s.as_str())).collect();

    let (content, errors) = core::read_filelists(
        &paths,
        &mut directives,
        !no_recursive,
        deduplication,
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
