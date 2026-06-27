use flatten_filelist::core;
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <filelist> [directive...]", args[0]);
        process::exit(1);
    }

    let filelist = &args[1];
    let mut directives: Vec<String> = args.iter().skip(2).cloned().collect();

    let (content, errors) = core::read_filelist(std::path::Path::new(filelist), &mut directives);

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
