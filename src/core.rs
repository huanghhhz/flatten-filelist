use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

pub fn env_decode(input: &str) -> Result<String, String> {
    if !input.contains('$') {
        return Ok(input.to_string());
    }
    let os_env: HashMap<String, String> = env::vars().collect();
    let mut arr: Vec<String> = input.split('/').map(|s| s.to_string()).collect();
    for segment in arr.iter_mut() {
        if segment.contains('$') {
            let parts: Vec<&str> = segment.split('$').collect();
            let var_name = parts[1].trim_matches(|c| c == '{' || c == '}');
            if let Some(val) = os_env.get(var_name) {
                *segment = format!("{}{}", parts[0], val);
            } else {
                return Err(format!("ENV_NOT_FOUND: {}", var_name));
            }
        }
    }
    Ok(arr.join("/"))
}

/// Filter filelist lines based on Verilog preprocessor directive conditionals.
/// Mutates `directives` in place when `define directives are encountered in active blocks.
pub fn directive_filter(lines: &[String], directives: &mut Vec<String>) -> Vec<String> {
    let mut output: Vec<String> = Vec::new();
    let mut macro_matched: Vec<Option<bool>> = vec![Some(true)];
    let all_matched = |stack: &[Option<bool>]| stack.iter().all(|b| *b == Some(true));
    let wildcard = directives.contains(&"*".to_string());

    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with('`') {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            match parts[0] {
                "`ifdef" => {
                    macro_matched.push(Some(directives.contains(&parts[1].to_string())));
                }
                "`ifndef" => {
                    macro_matched.push(Some(!directives.contains(&parts[1].to_string())));
                }
                "`elsif" => {
                    let last = macro_matched.last_mut().unwrap();
                    match last {
                        Some(true) | None => *last = None,
                        Some(false) => *last = Some(directives.contains(&parts[1].to_string())),
                    }
                }
                "`else" => {
                    let last = macro_matched.last_mut().unwrap();
                    match last {
                        Some(true) | None => *last = None,
                        Some(false) => *last = Some(true),
                    }
                }
                "`endif" => {
                    macro_matched.pop();
                }
                _ => {}
            }
        } else {
            let active = all_matched(&macro_matched) || wildcard;
            if active {
                output.push(line.clone());
                if !wildcard && (trimmed.starts_with("-def") || trimmed.starts_with("`define")) {
                    let def_parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if def_parts.len() >= 2 {
                        directives.push(def_parts[1].to_string());
                    }
                }
            }
        }
    }
    output
}

pub fn read_filelist(path: &Path, directives: &mut Vec<String>) -> (Vec<String>, Vec<String>) {
    let mut out_content: Vec<String> = Vec::new();
    let mut errs: Vec<String> = Vec::new();

    if !path.exists() {
        errs.push(format!("input filelist not found: {}", path.display()));
        return (out_content, errs);
    }

    let origin_content = match fs::read_to_string(path) {
        Ok(s) => s.lines().map(|l| l.to_string()).collect::<Vec<_>>(),
        Err(e) => {
            errs.push(format!("failed to read input filelist: {}. {}", path.display(), e));
            return (out_content, errs);
        }
    };

    let filtered = directive_filter(&origin_content, directives);

    for line in &filtered {
        let trimmed = line.trim();
        if trimmed.starts_with("-f ") {
            let sub_path_str = trimmed[3..].trim();
            match env_decode(sub_path_str) {
                Ok(decoded) => {
                    let sub_path = Path::new(&decoded);
                    let (sub_content, sub_errs) = read_filelist(sub_path, directives);
                    out_content.extend(sub_content);
                    errs.extend(sub_errs);
                }
                Err(e) => {
                    errs.push(e);
                }
            }
        } else {
            out_content.push(line.clone());
        }
    }

    (out_content, errs)
}
