use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

/// Source location within a filelist (1-based line number).
#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceLoc {
    file: PathBuf,
    line: usize,
}

impl SourceLoc {
    fn display(&self) -> String {
        format!("{}:{}", self.file.display(), self.line)
    }
}

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

/// Parse a filelist item line, returning the prefix and the path portion.
/// Recognizes `-v `, `-y `, and `+incdir+` prefixes.
fn parse_item_path(line: &str) -> (&str, &str) {
    if let Some(rest) = line.strip_prefix("-v ") {
        ("-v ", rest)
    } else if let Some(rest) = line.strip_prefix("-y ") {
        ("-y ", rest)
    } else if let Some(rest) = line.strip_prefix("+incdir+") {
        ("+incdir+", rest)
    } else {
        ("", line)
    }
}

/// Normalize `.` and `..` components in a path without touching the filesystem.
/// Does NOT resolve symlinks (unlike `canonicalize`).
fn normalize_path(path: &Path) -> PathBuf {
    let mut components: Vec<Component> = Vec::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                components.pop();
            }
            Component::CurDir => {}
            c => components.push(c),
        }
    }
    components.into_iter().collect()
}

/// Expand env vars in a path string, then resolve to an absolute path
/// (normalizing `.` and `..`) without following symlinks.
pub fn resolve_absolute(path_str: &str) -> Result<String, String> {
    let expanded = env_decode(path_str)?;
    let path = Path::new(&expanded);
    let abs = if path.is_absolute() {
        normalize_path(path)
    } else {
        let cwd = env::current_dir()
            .map_err(|e| format!("cannot get current dir: {}", e))?;
        normalize_path(&cwd.join(path))
    };
    Ok(abs.to_string_lossy().to_string())
}

/// Replace the prefix of `abs_path` that matches the value of `env_name`
/// with `$ENV_NAME`. If the path doesn't start with the env value, it is
/// returned unchanged.
pub fn encode_with_env_name(abs_path: &str, env_name: &str) -> Result<String, String> {
    let env_val = env::var(env_name)
        .map_err(|_| format!("ENV_NOT_FOUND: {}", env_name))?;
    let env_path = Path::new(&env_val);
    let abs = Path::new(abs_path);

    match abs.strip_prefix(env_path) {
        Ok(relative) => {
            if relative.as_os_str().is_empty() {
                Ok(format!("${}", env_name))
            } else {
                Ok(format!("${}/{}", env_name, relative.display()))
            }
        }
        Err(_) => Ok(abs_path.to_string()),
    }
}

/// Post-process content items: parse prefixes, expand env vars, resolve to
/// absolute paths, check existence, and optionally encode back with an env var.
/// Each item carries a SourceLoc for use in error messages.
fn process_content_items(
    items: Vec<(String, SourceLoc)>,
    check_exist: bool,
    resolve_path: bool,
    encode_with_env: Option<&str>,
) -> (Vec<String>, Vec<String>) {
    let mut processed: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    let effective_resolve = resolve_path || encode_with_env.is_some();
    let effective_check = check_exist || effective_resolve;

    for (item, loc) in items {
        // +libext+, +define+, and other + directives are opaque —
        // they are not file paths, so skip all processing
        if item.starts_with('+') && !item.starts_with("+incdir+") {
            processed.push(item);
            continue;
        }

        let (prefix, path_str) = parse_item_path(&item);

        // Step 1: Expand env vars in the path
        let expanded = match env_decode(path_str) {
            Ok(s) => s,
            Err(e) => {
                errors.push(format!("{} ({})", e, loc.file.display()));
                processed.push(item);
                continue;
            }
        };

        // Step 2: Resolve to absolute path if needed
        let final_path = if effective_resolve {
            match resolve_absolute(&expanded) {
                Ok(s) => s,
                Err(e) => {
                    errors.push(format!("{} ({})", e, loc.file.display()));
                    processed.push(item);
                    continue;
                }
            }
        } else {
            expanded
        };

        // Step 3: Check existence (keep item in output, report error if missing)
        if effective_check && !Path::new(&final_path).exists() {
            errors.push(format!("item not found: {} ({})", item, loc.display()));
        }

        // Step 4: Reconstruct the item
        let final_item = if let Some(env_name) = encode_with_env {
            match encode_with_env_name(&final_path, env_name) {
                Ok(encoded) => format!("{}{}", prefix, encoded),
                Err(e) => {
                    errors.push(format!("{} ({})", e, loc.file.display()));
                    format!("{}{}", prefix, final_path)
                }
            }
        } else if effective_resolve {
            format!("{}{}", prefix, final_path)
        } else {
            // check_exist only: keep original form
            item
        };

        processed.push(final_item);
    }

    (processed, errors)
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
        if trimmed.starts_with('`') && !trimmed.starts_with("`define") {
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
                if !wildcard && (trimmed.starts_with("-def") || trimmed.starts_with("`define")) {
                    let def_parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if def_parts.len() >= 2 {
                        directives.push(def_parts[1].to_string());
                    }
                } else {
                    output.push(line.clone());
                }
            }
        }
    }
    output
}

/// Like directive_filter but preserves original 1-based line numbers
/// for each output line.
fn directive_filter_with_lines(
    lines: &[String],
    directives: &mut Vec<String>,
) -> Vec<(String, usize)> {
    let mut output: Vec<(String, usize)> = Vec::new();
    let mut macro_matched: Vec<Option<bool>> = vec![Some(true)];
    let all_matched = |stack: &[Option<bool>]| stack.iter().all(|b| *b == Some(true));
    let wildcard = directives.contains(&"*".to_string());

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('`') && !trimmed.starts_with("`define") {
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
                if !wildcard && (trimmed.starts_with("-def") || trimmed.starts_with("`define")) {
                    let def_parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if def_parts.len() >= 2 {
                        directives.push(def_parts[1].to_string());
                    }
                } else {
                    output.push((line.clone(), idx + 1));
                }
            }
        }
    }
    output
}

pub fn read_filelists(
    paths: &[&Path],
    directives: &mut Vec<String>,
    recursive: bool,
    deduplicate: bool,
    check_exist: bool,
    resolve_path: bool,
    encode_with_env: Option<&str>,
) -> (Vec<String>, Vec<String>) {
    let mut all_content: Vec<String> = Vec::new();
    let mut all_errors: Vec<String> = Vec::new();
    for path in paths {
        let (content, errors) =
            read_filelist(path, directives, recursive, deduplicate, check_exist, resolve_path, encode_with_env);
        all_content.extend(content);
        all_errors.extend(errors);
    }
    let effective_resolve = resolve_path || encode_with_env.is_some();
    let effective_check = check_exist || effective_resolve;
    let effective_dedup = deduplicate || effective_check;
    if effective_dedup {
        dedup_vec(&mut all_content);
        dedup_vec(&mut all_errors);
    }
    (all_content, all_errors)
}

pub fn read_filelist(
    path: &Path,
    directives: &mut Vec<String>,
    recursive: bool,
    deduplicate: bool,
    check_exist: bool,
    resolve_path: bool,
    encode_with_env: Option<&str>,
) -> (Vec<String>, Vec<String>) {
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut cache: HashMap<PathBuf, (Vec<(String, SourceLoc)>, Vec<String>)> = HashMap::new();
    let (mut tagged_content, mut errors) = _read_filelist(path, directives, recursive, &mut visited, &mut cache, None);

    let effective_resolve = resolve_path || encode_with_env.is_some();
    let effective_check = check_exist || effective_resolve;
    let effective_dedup = deduplicate || effective_check;

    if effective_dedup {
        dedup_tagged_vec(&mut tagged_content);
        dedup_vec(&mut errors);
    }

    let mut content: Vec<String>;
    if effective_check {
        let (processed_content, processed_errors) =
            process_content_items(tagged_content, effective_check, effective_resolve, encode_with_env);
        content = processed_content;
        errors.extend(processed_errors);

        // Re-dedup after resolve/encode: different relative paths may
        // collapse to the same absolute/encoded path.
        if effective_resolve && effective_dedup {
            dedup_vec(&mut content);
            dedup_vec(&mut errors);
        }
    } else {
        // Strip source locs when returning content without post-processing
        content = tagged_content.into_iter().map(|(s, _)| s).collect();
    }

    (content, errors)
}

fn dedup_vec(v: &mut Vec<String>) {
    let mut seen = HashSet::new();
    v.retain(|item| seen.insert(item.clone()));
}

fn dedup_tagged_vec(v: &mut Vec<(String, SourceLoc)>) {
    let mut seen = HashSet::new();
    v.retain(|(item, _loc)| seen.insert(item.clone()));
}

fn resolve_item_relative_to_base(item: &str, base: &Path) -> String {
    if item.starts_with('+') && !item.starts_with("+incdir+") {
        return item.to_string();
    }
    let (prefix, path_str) = parse_item_path(item);
    if Path::new(path_str).is_absolute() {
        return item.to_string();
    }
    let resolved = normalize_path(&base.join(path_str));
    format!("{}{}", prefix, resolved.to_string_lossy())
}

fn resolve_canonical(path: &Path) -> PathBuf {
    match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            match resolve_absolute(&path.to_string_lossy()) {
                Ok(p) => PathBuf::from(p),
                Err(_) => path.to_path_buf(),
            }
        }
    }
}

fn _read_filelist(
    path: &Path,
    directives: &mut Vec<String>,
    recursive: bool,
    visited: &mut HashSet<PathBuf>,
    cache: &mut HashMap<PathBuf, (Vec<(String, SourceLoc)>, Vec<String>)>,
    base_dir: Option<&Path>,
) -> (Vec<(String, SourceLoc)>, Vec<String>) {
    let canonical = resolve_canonical(path);

    if base_dir.is_none() {
        if let Some((content, errs)) = cache.get(&canonical) {
            return (content.clone(), errs.clone());
        }
    }

    if !visited.insert(canonical.clone()) {
        let errs = vec![format!(
            "circular include detected: {}",
            path.display()
        )];
        return (Vec::new(), errs);
    }

    // Ensure we clean up visited on every return path
    let result = _read_filelist_impl(path, directives, recursive, visited, cache, base_dir);

    visited.remove(&canonical);

    if base_dir.is_none() {
        cache.insert(canonical, result.clone());
    }
    result
}

fn _read_filelist_impl(
    path: &Path,
    directives: &mut Vec<String>,
    recursive: bool,
    visited: &mut HashSet<PathBuf>,
    cache: &mut HashMap<PathBuf, (Vec<(String, SourceLoc)>, Vec<String>)>,
    base_dir: Option<&Path>,
) -> (Vec<(String, SourceLoc)>, Vec<String>) {
    let mut out_content: Vec<(String, SourceLoc)> = Vec::new();
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

    let filtered = directive_filter_with_lines(&origin_content, directives);

    for (line, line_num) in &filtered {
        let trimmed = line.trim();
        if recursive && trimmed.starts_with("-F ") {
            let sub_path_str = trimmed[3..].trim();
            match env_decode(sub_path_str) {
                Ok(decoded) => {
                    let sub_path = Path::new(&decoded);
                    let sub_base_dir = resolve_canonical(sub_path).parent().map(|p| p.to_path_buf());
                    let (sub_content, sub_errs) = _read_filelist(
                        sub_path, directives, recursive, visited, cache,
                        sub_base_dir.as_deref(),
                    );
                    out_content.extend(sub_content);
                    errs.extend(sub_errs);
                }
                Err(e) => {
                    errs.push(format!("{} ({})", e, path.display()));
                }
            }
        } else if recursive && trimmed.starts_with("-f ") {
            let sub_path_str = trimmed[3..].trim();
            match env_decode(sub_path_str) {
                Ok(decoded) => {
                    let sub_path = Path::new(&decoded);
                    let (sub_content, sub_errs) = _read_filelist(sub_path, directives, recursive, visited, cache, None);
                    out_content.extend(sub_content);
                    errs.extend(sub_errs);
                }
                Err(e) => {
                    errs.push(format!("{} ({})", e, path.display()));
                }
            }
        } else if !trimmed.is_empty() && !trimmed.starts_with("//") {
            let resolved = if let Some(base) = base_dir {
                resolve_item_relative_to_base(trimmed, base)
            } else {
                trimmed.to_string()
            };
            out_content.push((resolved, SourceLoc {
                file: path.to_path_buf(),
                line: *line_num,
            }));
        }
    }

    (out_content, errs)
}
