use flatten_filelist::core::directive_filter;
use flatten_filelist::core::env_decode;
use std::env;
use std::fs;

#[test]
fn test_env_decode_basic_expansion() {
    env::set_var("TEST_PROJ", "/home/user/project");
    let result = env_decode("$TEST_PROJ/src/file.v");
    assert_eq!(result, Ok("/home/user/project/src/file.v".to_string()));
    env::remove_var("TEST_PROJ");
}

#[test]
fn test_env_decode_missing_var() {
    env::remove_var("MISSING_VAR");
    let result = env_decode("$MISSING_VAR/file.v");
    assert_eq!(result, Err("ENV_NOT_FOUND: MISSING_VAR".to_string()));
}

#[test]
fn test_env_decode_nested_braces() {
    env::set_var("TEST_ROOT", "/opt/root");
    let result = env_decode("${TEST_ROOT}/lib/file.v");
    assert_eq!(result, Ok("/opt/root/lib/file.v".to_string()));
    env::remove_var("TEST_ROOT");
}

#[test]
fn test_env_decode_no_dollar_passthrough() {
    let result = env_decode("plain/path/file.v");
    assert_eq!(result, Ok("plain/path/file.v".to_string()));
}

#[test]
fn test_directive_filter_ifdef_matching() {
    let lines = vec![
        "file1.v".to_string(),
        "`ifdef FOO".to_string(),
        "file2.v".to_string(),
        "`endif".to_string(),
    ];
    let mut directives = vec!["FOO".to_string()];
    let filtered = directive_filter(&lines, &mut directives);
    assert_eq!(filtered, vec!["file1.v".to_string(), "file2.v".to_string()]);
}

#[test]
fn test_directive_filter_ifdef_not_matching() {
    let lines = vec![
        "file1.v".to_string(),
        "`ifdef BAR".to_string(),
        "file2.v".to_string(),
        "`endif".to_string(),
    ];
    let mut directives = vec!["FOO".to_string()];
    let filtered = directive_filter(&lines, &mut directives);
    assert_eq!(filtered, vec!["file1.v".to_string()]);
}

#[test]
fn test_directive_filter_ifndef() {
    let lines = vec![
        "`ifndef FOO".to_string(),
        "file1.v".to_string(),
        "`endif".to_string(),
    ];
    let mut directives = vec!["FOO".to_string()];
    let filtered = directive_filter(&lines, &mut directives);
    assert_eq!(filtered, Vec::<String>::new());
}

#[test]
fn test_directive_filter_else() {
    let lines = vec![
        "`ifdef FOO".to_string(),
        "file1.v".to_string(),
        "`else".to_string(),
        "file2.v".to_string(),
        "`endif".to_string(),
    ];
    let mut directives: Vec<String> = vec![];
    let filtered = directive_filter(&lines, &mut directives);
    assert_eq!(filtered, vec!["file2.v".to_string()]);
}

#[test]
fn test_directive_filter_nested() {
    let lines = vec![
        "`ifdef A".to_string(),
        "file1.v".to_string(),
        "`ifdef B".to_string(),
        "file2.v".to_string(),
        "`endif".to_string(),
        "`endif".to_string(),
    ];
    let mut directives = vec!["A".to_string()];
    let filtered = directive_filter(&lines, &mut directives);
    assert_eq!(filtered, vec!["file1.v".to_string()]);
}

#[test]
fn test_directive_filter_wildcard() {
    let lines = vec![
        "`ifdef FOO".to_string(),
        "file1.v".to_string(),
        "`endif".to_string(),
    ];
    let mut directives = vec!["*".to_string()];
    let filtered = directive_filter(&lines, &mut directives);
    assert_eq!(filtered, vec!["file1.v".to_string()]);
}

#[test]
fn test_directive_filter_define_accumulation() {
    let lines = vec![
        "`define BAR".to_string(),
        "`ifdef BAR".to_string(),
        "file1.v".to_string(),
        "`endif".to_string(),
    ];
    let mut directives: Vec<String> = vec![];
    let filtered = directive_filter(&lines, &mut directives);
    assert_eq!(filtered, vec!["file1.v".to_string()]);
    assert!(directives.contains(&"BAR".to_string()));
}

#[test]
fn test_directive_filter_elsif() {
    let lines = vec![
        "`ifdef FOO".to_string(),
        "file1.v".to_string(),
        "`elsif BAR".to_string(),
        "file2.v".to_string(),
        "`else".to_string(),
        "file3.v".to_string(),
        "`endif".to_string(),
    ];
    let mut directives = vec!["BAR".to_string()];
    let filtered = directive_filter(&lines, &mut directives);
    assert_eq!(filtered, vec!["file2.v".to_string()]);
}

#[test]
fn test_directive_filter_elsif_first_branch_matches() {
    let lines = vec![
        "`ifdef FOO".to_string(),
        "file1.v".to_string(),
        "`elsif BAR".to_string(),
        "file2.v".to_string(),
        "`else".to_string(),
        "file3.v".to_string(),
        "`endif".to_string(),
    ];
    let mut directives = vec!["FOO".to_string()];
    let filtered = directive_filter(&lines, &mut directives);
    assert_eq!(filtered, vec!["file1.v".to_string()]);
}

#[test]
fn test_directive_filter_elsif_else_branch() {
    let lines = vec![
        "`ifdef FOO".to_string(),
        "file1.v".to_string(),
        "`elsif BAR".to_string(),
        "file2.v".to_string(),
        "`else".to_string(),
        "file3.v".to_string(),
        "`endif".to_string(),
    ];
    let mut directives: Vec<String> = vec![];
    let filtered = directive_filter(&lines, &mut directives);
    assert_eq!(filtered, vec!["file3.v".to_string()]);
}

use flatten_filelist::core::read_filelist;
use flatten_filelist::core::read_filelists;
use std::path::Path;
use tempfile::NamedTempFile;
use std::io::Write;

#[test]
fn test_read_filelist_single_file() {
    let mut tmp = NamedTempFile::new().unwrap();
    writeln!(tmp, "src/file1.v").unwrap();
    writeln!(tmp, "src/file2.v").unwrap();
    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(tmp.path(), &mut directives, true, true, false, false, None);
    assert_eq!(content, vec!["src/file1.v".to_string(), "src/file2.v".to_string()]);
    assert!(errors.is_empty());
}

#[test]
fn test_read_filelist_nested_include() {
    let mut sub = NamedTempFile::new().unwrap();
    writeln!(sub, "sub/file.v").unwrap();
    let sub_path = sub.path().to_str().unwrap().to_string();

    let mut main = NamedTempFile::new().unwrap();
    writeln!(main, "main/file1.v").unwrap();
    writeln!(main, "-f {}", sub_path).unwrap();
    writeln!(main, "main/file2.v").unwrap();

    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(main.path(), &mut directives, true, true, false, false, None);
    assert_eq!(content, vec![
        "main/file1.v".to_string(),
        "sub/file.v".to_string(),
        "main/file2.v".to_string(),
    ]);
    assert!(errors.is_empty());
}

#[test]
fn test_read_filelist_missing_file() {
    let path = Path::new("/nonexistent/filelist.f");
    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(path, &mut directives, true, true, false, false, None);
    assert!(content.is_empty());
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("not found"));
}

#[test]
fn test_read_filelist_env_var_in_include() {
    env::set_var("TEST_SUB_DIR", "/tmp");
    let mut sub = NamedTempFile::new_in("/tmp").unwrap();
    writeln!(sub, "sub/file.v").unwrap();
    let sub_name = sub.path().file_name().unwrap().to_str().unwrap();

    let mut main = NamedTempFile::new().unwrap();
    writeln!(main, "-f $TEST_SUB_DIR/{}", sub_name).unwrap();

    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(main.path(), &mut directives, true, true, false, false, None);
    assert_eq!(content, vec!["sub/file.v".to_string()]);
    assert!(errors.is_empty());
    env::remove_var("TEST_SUB_DIR");
}

#[test]
fn test_read_filelist_missing_env_var_in_include() {
    env::remove_var("MISSING_DIR");
    let mut main = NamedTempFile::new().unwrap();
    writeln!(main, "-f $MISSING_DIR/filelist.f").unwrap();

    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(main.path(), &mut directives, true, true, false, false, None);
    assert!(content.is_empty());
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("ENV_NOT_FOUND"));
}

#[test]
fn test_read_filelist_no_recursive() {
    let mut sub = NamedTempFile::new().unwrap();
    writeln!(sub, "sub/file.v").unwrap();
    let sub_path = sub.path().to_str().unwrap().to_string();

    let mut main = NamedTempFile::new().unwrap();
    writeln!(main, "main/file1.v").unwrap();
    writeln!(main, "-f {}", sub_path).unwrap();
    writeln!(main, "main/file2.v").unwrap();

    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(main.path(), &mut directives, false, true, false, false, None);
    assert_eq!(content, vec![
        "main/file1.v".to_string(),
        format!("-f {}", sub_path),
        "main/file2.v".to_string(),
    ]);
    assert!(errors.is_empty());
}

#[test]
fn test_read_filelists_multiple_files() {
    let mut f1 = NamedTempFile::new().unwrap();
    writeln!(f1, "a/file1.v").unwrap();
    writeln!(f1, "common/dup.v").unwrap();

    let mut f2 = NamedTempFile::new().unwrap();
    writeln!(f2, "common/dup.v").unwrap();
    writeln!(f2, "b/file3.v").unwrap();

    let mut directives: Vec<String> = vec![];
    let paths: Vec<&Path> = vec![f1.path(), f2.path()];
    let (content, errors) = read_filelists(&paths, &mut directives, true, true, false, false, None);
    assert_eq!(content, vec![
        "a/file1.v".to_string(),
        "common/dup.v".to_string(),
        "b/file3.v".to_string(),
    ]);
    assert!(errors.is_empty());
}

#[test]
fn test_read_filelists_shared_directives() {
    let mut f1 = NamedTempFile::new().unwrap();
    writeln!(f1, "`define FOO").unwrap();

    let mut f2 = NamedTempFile::new().unwrap();
    writeln!(f2, "`ifdef FOO").unwrap();
    writeln!(f2, "shared/file.v").unwrap();
    writeln!(f2, "`endif").unwrap();

    let mut directives: Vec<String> = vec![];
    let paths: Vec<&Path> = vec![f1.path(), f2.path()];
    let (content, errors) = read_filelists(&paths, &mut directives, true, true, false, false, None);
    assert_eq!(content, vec!["shared/file.v".to_string()]);
    assert!(errors.is_empty());
}

// --- tests for parse_item_path, normalize_path, resolve_absolute, encode_with_env_name ---

use flatten_filelist::core::resolve_absolute;
use flatten_filelist::core::encode_with_env_name;
use std::path::PathBuf;

#[test]
fn test_resolve_absolute_relative() {
    let cwd = env::current_dir().unwrap();
    let result = resolve_absolute("src/core.rs").unwrap();
    let expected = cwd.join("src/core.rs");
    assert_eq!(PathBuf::from(&result), normalize_path_in_test(&expected));
}

#[test]
fn test_resolve_absolute_with_dotdot() {
    let cwd = env::current_dir().unwrap();
    let result = resolve_absolute("src/../src/core.rs").unwrap();
    let expected = cwd.join("src/core.rs");
    assert_eq!(PathBuf::from(&result), normalize_path_in_test(&expected));
}

#[test]
fn test_resolve_absolute_with_env() {
    env::set_var("TEST_ABS_DIR", "/tmp");
    let result = resolve_absolute("$TEST_ABS_DIR/sub/file.v").unwrap();
    assert_eq!(result, "/tmp/sub/file.v");
    env::remove_var("TEST_ABS_DIR");
}

#[test]
fn test_resolve_absolute_missing_env() {
    env::remove_var("MISSING_ABS_VAR");
    let result = resolve_absolute("$MISSING_ABS_VAR/file.v");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("ENV_NOT_FOUND"));
}

#[test]
fn test_encode_with_env_name_basic() {
    env::set_var("TEST_ENV", "/home/user/proj");
    let result = encode_with_env_name("/home/user/proj/sub/file.v", "TEST_ENV").unwrap();
    assert_eq!(result, "$TEST_ENV/sub/file.v");
    env::remove_var("TEST_ENV");
}

#[test]
fn test_encode_with_env_name_exact_match() {
    env::set_var("TEST_ENV2", "/opt/data");
    let result = encode_with_env_name("/opt/data", "TEST_ENV2").unwrap();
    assert_eq!(result, "$TEST_ENV2");
    env::remove_var("TEST_ENV2");
}

#[test]
fn test_encode_with_env_name_no_match() {
    env::set_var("TEST_ENV3", "/other/path");
    let result = encode_with_env_name("/home/user/file.v", "TEST_ENV3").unwrap();
    assert_eq!(result, "/home/user/file.v");
    env::remove_var("TEST_ENV3");
}

#[test]
fn test_encode_with_env_name_missing_env() {
    env::remove_var("MISSING_ENV");
    let result = encode_with_env_name("/some/path", "MISSING_ENV");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("ENV_NOT_FOUND"));
}

fn normalize_path_in_test(path: &Path) -> PathBuf {
    let mut components: Vec<std::path::Component> = Vec::new();
    for c in path.components() {
        match c {
            std::path::Component::ParentDir => { components.pop(); }
            std::path::Component::CurDir => {}
            _ => components.push(c),
        }
    }
    components.into_iter().collect()
}

// --- integration tests for check_exist / resolve_path / encode_with_env ---

#[test]
fn test_check_exist_existing_file() {
    let mut tmp = NamedTempFile::new().unwrap();
    writeln!(tmp, "src/core.rs").unwrap();
    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(tmp.path(), &mut directives, true, true, true, false, None);
    // src/core.rs should exist relative to cwd
    assert!(content.contains(&"src/core.rs".to_string()));
    assert!(errors.is_empty());
}

#[test]
fn test_check_exist_missing_item() {
    let mut tmp = NamedTempFile::new().unwrap();
    writeln!(tmp, "/nonexistent/file_xyz123.v").unwrap();
    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(tmp.path(), &mut directives, true, true, true, false, None);
    assert!(content.contains(&"/nonexistent/file_xyz123.v".to_string()));
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("not found"));
}

#[test]
fn test_resolve_path_absolute() {
    let mut tmp = NamedTempFile::new().unwrap();
    writeln!(tmp, "src/core.rs").unwrap();
    let cwd = env::current_dir().unwrap();
    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(tmp.path(), &mut directives, true, true, false, true, None);
    let expected = normalize_path_in_test(&cwd.join("src/core.rs"));
    assert_eq!(content, vec![expected.to_string_lossy().to_string()]);
    assert!(errors.is_empty());
}

#[test]
fn test_encode_with_env_integration() {
    env::set_var("TEST_FLAT_ENV", "/tmp");
    let mut tmp = NamedTempFile::new_in("/tmp").unwrap();
    let fname = tmp.path().file_name().unwrap().to_str().unwrap().to_string();
    writeln!(tmp, "/tmp/{}", fname).unwrap();
    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(
        tmp.path(), &mut directives, true, true, false, false, Some("TEST_FLAT_ENV"),
    );
    assert_eq!(content, vec![format!("$TEST_FLAT_ENV/{}", fname)]);
    assert!(errors.is_empty());
    env::remove_var("TEST_FLAT_ENV");
}

#[test]
fn test_check_exist_with_prefix_v() {
    // Use an existing file with -v prefix
    let mut tmp = NamedTempFile::new().unwrap();
    let cwd = env::current_dir().unwrap();
    let existing = cwd.join("src/core.rs");
    writeln!(tmp, "-v {}", existing.display()).unwrap();
    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(tmp.path(), &mut directives, true, true, true, false, None);
    assert!(errors.is_empty());
    // check_exist only: prefix and path kept as-is
    assert!(content[0].starts_with("-v "));
}

#[test]
fn test_check_exist_with_incdir() {
    let mut tmp = NamedTempFile::new().unwrap();
    let cwd = env::current_dir().unwrap();
    let existing_dir = cwd.join("src");
    writeln!(tmp, "+incdir+{}", existing_dir.display()).unwrap();
    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(tmp.path(), &mut directives, true, true, true, false, None);
    assert!(errors.is_empty());
    assert!(content[0].starts_with("+incdir+"));
}

#[test]
fn test_libext_passthrough_no_check() {
    // +libext+ is not a file path — pass through untouched, no existence error
    let mut tmp = NamedTempFile::new().unwrap();
    writeln!(tmp, "+libext+.v+.sv").unwrap();
    writeln!(tmp, "+define+FOO=1").unwrap();
    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(tmp.path(), &mut directives, true, true, true, false, None);
    assert_eq!(content, vec!["+libext+.v+.sv".to_string(), "+define+FOO=1".to_string()]);
    assert!(errors.is_empty());
}

#[test]
fn test_re_dedup_after_resolve() {
    // Two different relative paths that resolve to the same absolute path
    // should be deduplicated after resolve_path
    let mut tmp = NamedTempFile::new().unwrap();
    writeln!(tmp, "src/core.rs").unwrap();
    writeln!(tmp, "src/../src/core.rs").unwrap();
    let cwd = env::current_dir().unwrap();
    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(tmp.path(), &mut directives, true, true, false, true, None);
    let expected = normalize_path_in_test(&cwd.join("src/core.rs"));
    assert_eq!(content, vec![expected.to_string_lossy().to_string()]);
    assert!(errors.is_empty());
}

#[test]
fn test_dedup_single_filelist_duplicate_line() {
    // Same line appearing twice in one filelist → appears once
    let mut tmp = NamedTempFile::new().unwrap();
    writeln!(tmp, "src/file.v").unwrap();
    writeln!(tmp, "src/other.v").unwrap();
    writeln!(tmp, "src/file.v").unwrap(); // duplicate
    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(tmp.path(), &mut directives, true, true, false, false, None);
    assert_eq!(content, vec!["src/file.v".to_string(), "src/other.v".to_string()]);
    assert!(errors.is_empty());
}

#[test]
fn test_dedup_nested_include_same_file() {
    // Same file reached via two different -f include chains → appears once
    let mut shared = NamedTempFile::new().unwrap();
    writeln!(shared, "shared/file.v").unwrap();
    let shared_path = shared.path().to_str().unwrap().to_string();

    let mut sub_a = NamedTempFile::new().unwrap();
    writeln!(sub_a, "-f {}", shared_path).unwrap();
    let sub_a_path = sub_a.path().to_str().unwrap().to_string();

    let mut sub_b = NamedTempFile::new().unwrap();
    writeln!(sub_b, "-f {}", shared_path).unwrap();
    let sub_b_path = sub_b.path().to_str().unwrap().to_string();

    let mut main = NamedTempFile::new().unwrap();
    writeln!(main, "main/top.v").unwrap();
    writeln!(main, "-f {}", sub_a_path).unwrap();
    writeln!(main, "-f {}", sub_b_path).unwrap();

    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(main.path(), &mut directives, true, true, false, false, None);
    assert_eq!(content, vec![
        "main/top.v".to_string(),
        "shared/file.v".to_string(),
    ]);
    assert!(errors.is_empty());
}

#[test]
fn test_dedup_after_encode_with_env() {
    // Two identical paths that both encode to the same $ENV/... string
    // after encoding → only one entry
    env::set_var("TEST_DEDUP_ENV", "/tmp");
    let mut tmp = NamedTempFile::new_in("/tmp").unwrap();
    let fname = tmp.path().file_name().unwrap().to_str().unwrap().to_string();
    // Write the same path twice — both resolve+encode to the same string
    writeln!(tmp, "/tmp/{}", fname).unwrap();
    writeln!(tmp, "/tmp/{}", fname).unwrap();
    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(
        tmp.path(), &mut directives, true, true, false, false, Some("TEST_DEDUP_ENV"),
    );
    assert_eq!(content, vec![format!("$TEST_DEDUP_ENV/{}", fname)]);
    assert!(errors.is_empty());
    env::remove_var("TEST_DEDUP_ENV");
}

#[test]
fn test_circular_include_detection() {
    // a.f -> b.f -> a.f should be detected and reported, not stack-overflow
    let mut a = NamedTempFile::new().unwrap();
    let a_path = a.path().to_str().unwrap().to_string();
    let mut b = NamedTempFile::new().unwrap();
    let b_path = b.path().to_str().unwrap().to_string();

    writeln!(a, "-f {}", b_path).unwrap();
    writeln!(b, "-f {}", a_path).unwrap();

    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(a.path(), &mut directives, true, true, false, false, None);
    // Should not crash; should report circular include error
    assert!(content.is_empty());
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("circular include detected"));
}

#[test]
fn test_self_include_detection() {
    // a.f includes itself via -f
    let mut a = NamedTempFile::new().unwrap();
    let a_path = a.path().to_str().unwrap().to_string();
    writeln!(a, "top/file.v").unwrap();
    writeln!(a, "-f {}", a_path).unwrap();

    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(a.path(), &mut directives, true, true, false, false, None);
    // top/file.v appears once (before the self-include), self-include is caught
    assert_eq!(content, vec!["top/file.v".to_string()]);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("circular include detected"));
}

#[test]
fn test_no_dedup_when_all_flags_off() {
    // When deduplication=false and no other flags force it,
    // duplicate lines are kept in output
    let mut tmp = NamedTempFile::new().unwrap();
    writeln!(tmp, "src/file.v").unwrap();
    writeln!(tmp, "src/other.v").unwrap();
    writeln!(tmp, "src/file.v").unwrap(); // duplicate — should stay
    let mut directives: Vec<String> = vec![];
    // deduplication=false, check_exist=false, resolve_path=false, encode_with_env=None
    let (content, errors) = read_filelist(tmp.path(), &mut directives, true, false, false, false, None);
    assert_eq!(content, vec![
        "src/file.v".to_string(),
        "src/other.v".to_string(),
        "src/file.v".to_string(), // duplicate preserved
    ]);
    assert!(errors.is_empty());
}

#[test]
fn test_dedup_auto_on_with_check_exist() {
    // check_exist=true auto-forces deduplication=true via cascade
    let mut tmp = NamedTempFile::new().unwrap();
    writeln!(tmp, "src/core.rs").unwrap();
    writeln!(tmp, "src/core.rs").unwrap(); // duplicate
    let mut directives: Vec<String> = vec![];
    // deduplication=false explicitly, but check_exist=true → effective_dedup=true
    let (content, errors) = read_filelist(tmp.path(), &mut directives, true, false, true, false, None);
    // src/core.rs exists, and duplicates are removed (auto-dedup)
    assert_eq!(content, vec!["src/core.rs".to_string()]);
    assert!(errors.is_empty());
}

// --- error deduplication tests ---

#[test]
fn test_error_dedup_duplicate_env_failure_in_includes() {
    // Two identical -f lines with unresolved $ENV in the same file.
    // ENV_NOT_FOUND errors don't carry line numbers → identical
    // strings → deduped to 1.
    env::remove_var("MISSING_ERR_DEDUP_A");
    let mut tmp = NamedTempFile::new().unwrap();
    writeln!(tmp, "-f $MISSING_ERR_DEDUP_A/sub.f").unwrap();
    writeln!(tmp, "-f $MISSING_ERR_DEDUP_A/sub.f").unwrap();
    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(
        tmp.path(), &mut directives, true, true, false, false, None,
    );
    assert!(content.is_empty());
    // Same missing env → same error message → deduped to 1
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("ENV_NOT_FOUND"));
}

#[test]
fn test_error_dedup_missing_item_across_filelists() {
    // Two filelists, each with the same missing path. With source locs
    // in errors, they carry different file names → not deduped → 2 errors.
    let mut f1 = NamedTempFile::new().unwrap();
    writeln!(f1, "/nonexistent/file_err_dedup.v").unwrap();
    let mut f2 = NamedTempFile::new().unwrap();
    writeln!(f2, "/nonexistent/file_err_dedup.v").unwrap();
    let mut directives: Vec<String> = vec![];
    let paths: Vec<&Path> = vec![f1.path(), f2.path()];
    let (content, errors) = read_filelists(
        &paths, &mut directives, true, false, true, false, None,
    );
    // Content: each filelist contributes the missing path; cross-filelist dedup keeps one
    assert_eq!(content, vec!["/nonexistent/file_err_dedup.v".to_string()]);
    // Errors: different file names in source loc → 2 errors, not deduped
    assert_eq!(errors.len(), 2);
    assert!(errors[0].contains("not found"));
    assert!(errors[0].contains(":1)"));
    assert!(errors[1].contains("not found"));
    assert!(errors[1].contains(":1)"));
}

#[test]
fn test_error_dedup_sub_file_error_across_filelists() {
    // Two top-level filelists each -f include the same sub-filelist
    // that itself has an unresolved $ENV. Each read_filelist call
    // independently parses the sub-file, producing the same error.
    // read_filelists dedup → 1.
    env::remove_var("MISSING_ERR_DEDUP_C");
    let mut sub = NamedTempFile::new().unwrap();
    writeln!(sub, "-f $MISSING_ERR_DEDUP_C/subsub.f").unwrap();
    let sub_path = sub.path().to_str().unwrap().to_string();

    let mut a = NamedTempFile::new().unwrap();
    writeln!(a, "top/a.v").unwrap();
    writeln!(a, "-f {}", sub_path).unwrap();
    let mut b = NamedTempFile::new().unwrap();
    writeln!(b, "top/b.v").unwrap();
    writeln!(b, "-f {}", sub_path).unwrap();

    let mut directives: Vec<String> = vec![];
    let paths: Vec<&Path> = vec![a.path(), b.path()];
    let (content, errors) = read_filelists(
        &paths, &mut directives, true, true, false, false, None,
    );
    assert_eq!(content, vec!["top/a.v".to_string(), "top/b.v".to_string()]);
    // The sub-file's env error appears in each read_filelist call;
    // same file + same line → identical error string → deduped to 1.
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("ENV_NOT_FOUND"));
}

#[test]
fn test_no_dedup_errors_when_flags_off() {
    // deduplicate=false and no other flags → errors NOT deduped
    env::remove_var("MISSING_ERR_DEDUP_D");
    let mut tmp = NamedTempFile::new().unwrap();
    writeln!(tmp, "-f $MISSING_ERR_DEDUP_D/sub.f").unwrap();
    writeln!(tmp, "-f $MISSING_ERR_DEDUP_D/sub.f").unwrap();
    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(
        tmp.path(), &mut directives, true, false, false, false, None,
    );
    assert!(content.is_empty());
    // deduplicate=false → both identical errors are preserved
    assert_eq!(errors.len(), 2);
    assert!(errors[0].contains("ENV_NOT_FOUND"));
    assert!(errors[1].contains("ENV_NOT_FOUND"));
}

// --- tests for -F (resolve paths relative to sub-filelist) ---

#[test]
fn test_f_include_relative_paths_resolved() {
    let mut sub = NamedTempFile::new().unwrap();
    writeln!(sub, "file.v").unwrap();
    let sub_path = sub.path().to_str().unwrap().to_string();
    let sub_dir = sub.path().parent().unwrap();

    let mut main = NamedTempFile::new().unwrap();
    writeln!(main, "-F {}", sub_path).unwrap();

    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(main.path(), &mut directives, true, true, false, false, None);
    let expected = normalize_path_in_test(&sub_dir.join("file.v"));
    assert_eq!(content, vec![expected.to_string_lossy().to_string()]);
    assert!(errors.is_empty());
}

#[test]
fn test_f_include_with_prefix_v() {
    let mut sub = NamedTempFile::new().unwrap();
    writeln!(sub, "-v file.v").unwrap();
    let sub_path = sub.path().to_str().unwrap().to_string();
    let sub_dir = sub.path().parent().unwrap();

    let mut main = NamedTempFile::new().unwrap();
    writeln!(main, "-F {}", sub_path).unwrap();

    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(main.path(), &mut directives, true, true, false, false, None);
    let expected = normalize_path_in_test(&sub_dir.join("file.v"));
    assert_eq!(content, vec![format!("-v {}", expected.to_string_lossy())]);
    assert!(errors.is_empty());
}

#[test]
fn test_f_include_with_prefix_y() {
    let mut sub = NamedTempFile::new().unwrap();
    writeln!(sub, "-y file.v").unwrap();
    let sub_path = sub.path().to_str().unwrap().to_string();
    let sub_dir = sub.path().parent().unwrap();

    let mut main = NamedTempFile::new().unwrap();
    writeln!(main, "-F {}", sub_path).unwrap();

    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(main.path(), &mut directives, true, true, false, false, None);
    let expected = normalize_path_in_test(&sub_dir.join("file.v"));
    assert_eq!(content, vec![format!("-y {}", expected.to_string_lossy())]);
    assert!(errors.is_empty());
}

#[test]
fn test_f_include_with_incdir() {
    let mut sub = NamedTempFile::new().unwrap();
    writeln!(sub, "+incdir+dir").unwrap();
    let sub_path = sub.path().to_str().unwrap().to_string();
    let sub_dir = sub.path().parent().unwrap();

    let mut main = NamedTempFile::new().unwrap();
    writeln!(main, "-F {}", sub_path).unwrap();

    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(main.path(), &mut directives, true, true, false, false, None);
    let expected = normalize_path_in_test(&sub_dir.join("dir"));
    assert_eq!(content, vec![format!("+incdir+{}", expected.to_string_lossy())]);
    assert!(errors.is_empty());
}

#[test]
fn test_f_include_absolute_paths_unchanged() {
    let mut sub = NamedTempFile::new().unwrap();
    writeln!(sub, "/absolute/path/to/file.v").unwrap();
    let sub_path = sub.path().to_str().unwrap().to_string();

    let mut main = NamedTempFile::new().unwrap();
    writeln!(main, "-F {}", sub_path).unwrap();

    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(main.path(), &mut directives, true, true, false, false, None);
    assert_eq!(content, vec!["/absolute/path/to/file.v".to_string()]);
    assert!(errors.is_empty());
}

#[test]
fn test_f_include_opaque_plus_directives_unchanged() {
    let mut sub = NamedTempFile::new().unwrap();
    writeln!(sub, "+libext+.v").unwrap();
    writeln!(sub, "+define+MACRO").unwrap();
    let sub_path = sub.path().to_str().unwrap().to_string();

    let mut main = NamedTempFile::new().unwrap();
    writeln!(main, "-F {}", sub_path).unwrap();

    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(main.path(), &mut directives, true, true, false, false, None);
    assert_eq!(content, vec!["+libext+.v".to_string(), "+define+MACRO".to_string()]);
    assert!(errors.is_empty());
}

#[test]
fn test_f_include_nested_with_different_base_dirs() {
    let sub_dir = tempfile::tempdir().unwrap();
    let inner_sub_dir = sub_dir.path().join("inner");
    fs::create_dir(&inner_sub_dir).unwrap();

    let mut deep = NamedTempFile::new_in(&inner_sub_dir).unwrap();
    writeln!(deep, "deep_file.v").unwrap();
    let deep_path = deep.path().to_str().unwrap().to_string();

    let mut mid = NamedTempFile::new_in(&sub_dir).unwrap();
    writeln!(mid, "mid_file.v").unwrap();
    writeln!(mid, "-F {}", deep_path).unwrap();
    let mid_path = mid.path().to_str().unwrap().to_string();

    let mut main = NamedTempFile::new().unwrap();
    writeln!(main, "main_file.v").unwrap();
    writeln!(main, "-F {}", mid_path).unwrap();

    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(main.path(), &mut directives, true, true, false, false, None);

    let expected_mid = normalize_path_in_test(&sub_dir.path().join("mid_file.v"));
    let expected_deep = normalize_path_in_test(&inner_sub_dir.join("deep_file.v"));
    assert_eq!(content, vec![
        "main_file.v".to_string(),
        expected_mid.to_string_lossy().to_string(),
        expected_deep.to_string_lossy().to_string(),
    ]);
    assert!(errors.is_empty());
}

#[test]
fn test_f_include_mixed_with_f() {
    // -F includes resolve relative; -f includes do not
    let mut f_sub = NamedTempFile::new().unwrap();
    writeln!(f_sub, "f_relative.v").unwrap();
    let f_sub_path = f_sub.path().to_str().unwrap().to_string();

    let mut f_sub2 = NamedTempFile::new().unwrap();
    writeln!(f_sub2, "F_relative.v").unwrap();
    let f_sub2_path = f_sub2.path().to_str().unwrap().to_string();
    let f_sub2_dir = f_sub2.path().parent().unwrap();

    let mut main = NamedTempFile::new().unwrap();
    writeln!(main, "-f {}", f_sub_path).unwrap();
    writeln!(main, "-F {}", f_sub2_path).unwrap();

    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(main.path(), &mut directives, true, true, false, false, None);
    let expected_f2 = normalize_path_in_test(&f_sub2_dir.join("F_relative.v"));
    assert_eq!(content, vec![
        "f_relative.v".to_string(),
        expected_f2.to_string_lossy().to_string(),
    ]);
    assert!(errors.is_empty());
}

#[test]
fn test_f_include_no_recursive_treated_as_literal() {
    let mut sub = NamedTempFile::new().unwrap();
    writeln!(sub, "file.v").unwrap();
    let sub_path = sub.path().to_str().unwrap().to_string();

    let mut main = NamedTempFile::new().unwrap();
    writeln!(main, "main.v").unwrap();
    writeln!(main, "-F {}", sub_path).unwrap();

    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(main.path(), &mut directives, false, true, false, false, None);
    assert_eq!(content, vec![
        "main.v".to_string(),
        format!("-F {}", sub_path),
    ]);
    assert!(errors.is_empty());
}

#[test]
fn test_f_include_circular_detection() {
    let mut a = NamedTempFile::new().unwrap();
    let a_path = a.path().to_str().unwrap().to_string();
    writeln!(a, "top/file.v").unwrap();
    writeln!(a, "-F {}", a_path).unwrap();

    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(a.path(), &mut directives, true, true, false, false, None);
    assert_eq!(content, vec!["top/file.v".to_string()]);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("circular include detected"));
}

#[test]
fn test_f_include_with_dotdot() {
    let mut sub = NamedTempFile::new().unwrap();
    writeln!(sub, "../sibling.v").unwrap();
    let sub_path = sub.path().to_str().unwrap().to_string();
    let sub_dir = sub.path().parent().unwrap();

    let mut main = NamedTempFile::new().unwrap();
    writeln!(main, "-F {}", sub_path).unwrap();

    let mut directives: Vec<String> = vec![];
    let (content, errors) = read_filelist(main.path(), &mut directives, true, true, false, false, None);
    let expected = normalize_path_in_test(&sub_dir.join("../sibling.v"));
    assert_eq!(content, vec![expected.to_string_lossy().to_string()]);
    assert!(errors.is_empty());
}
