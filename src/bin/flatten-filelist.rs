use flatten_filelist::core;
use std::env;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let mut no_recursive = false;
    let mut filelists: Vec<String> = Vec::new();
    let mut directives: Vec<String> = Vec::new();
    let mut collecting_directives = false;

    for arg in &args {
        match arg.as_str() {
            "--no-recursive" if !collecting_directives => no_recursive = true,
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
        eprintln!("Usage: {} [--no-recursive] <filelist...> [-d DEFINE...]", env::args().next().unwrap_or_else(|| "flatten-filelist".into()));
        process::exit(1);
    }

    let paths: Vec<&Path> = filelists.iter().map(|s| Path::new(s.as_str())).collect();

    let (content, errors) = core::read_filelists(&paths, &mut directives, !no_recursive);

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
