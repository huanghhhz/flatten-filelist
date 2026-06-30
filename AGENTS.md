# AGENTS.md — flatten-filelist

Rust library + CLI + Python package that flattens Verilog filelists: resolves `-f` includes, expands `$VAR`/`${VAR}`, filters `\`ifdef`/`\`ifndef`/`\`elsif`/`\`else`/`\`endif` conditionals. Python package bundles a statically-linked musl binary (no PyO3 extension).

## Commands

```bash
cargo build --release --no-default-features --target x86_64-unknown-linux-musl   # static binary
cargo test                                                                       # Rust tests (45)

# Package: MUST copy binary before uv build
cp target/x86_64-unknown-linux-musl/release/flatten-filelist flatten_filelist/_bin/
uv build --wheel
uv venv && uv pip install dist/flatten_filelist-*.whl pytest
.venv/bin/python -m pytest pytests/ -v                                           # Python tests (15)
```

Always `--no-default-features --target x86_64-unknown-linux-musl`. musl static binary runs on any Linux kernel ≥2.6.

## Architecture

```
src/core.rs              — all business logic (public + private fns)
src/bin/flatten-filelist.rs  — CLI
src/lib.rs, src/bindings.rs  — legacy PyO3 (feature-gated, unused in packaging)
flatten_filelist/__init__.py — Python wrapper, subprocess.run the binary
flatten_filelist/_bin/       — committed musl binary, copied from target/ before uv build
tests/test_core.rs           — 45 Rust unit tests
pytests/                     — 15 Python integration tests
```

Hatchling bundles `flatten_filelist/_bin/flatten-filelist` into the wheel via `[tool.hatch.build.targets.wheel] artifacts`. `build/` is ephemeral (gitignored). The `target/` binary is the source of truth; `flatten_filelist/_bin/` is what gets packaged.

## Core functions (`src/core.rs`)

| Function | Signature | Role |
|---|---|---|
| `env_decode` | `(&str) -> Result<String, String>` | Expand `$VAR`/`${VAR}`; `Err("ENV_NOT_FOUND: <name>")` |
| `directive_filter` | `(&[String], &mut Vec<String>) -> Vec<String>` | Filter conditionals; ` `define `/`-def` skip the directive parser, accumulate in active blocks |
| `read_filelists` | `(&[&Path], &mut Vec<String>, recursive, deduplicate, check_exist, resolve_path, encode_with_env) -> (Vec<String>, Vec<String>)` | Multi-filelist entry point |
| `read_filelist` | `(&Path, &mut Vec<String>, recursive, deduplicate, check_exist, resolve_path, encode_with_env) -> (Vec<String>, Vec<String>)` | Single filelist: creates visited+cache → read → dedup → post-process |
| `_read_filelist` | `(&Path, &mut Vec<String>, recursive, &mut HashSet<PathBuf>, &mut HashMap<PathBuf, (Vec<String>, Vec<String>)>) -> (Vec<String>, Vec<String>)` | Cache check → cycle guard → delegate → cache/store result |
| `_read_filelist_impl` | same | Actual read: open file, directive_filter, resolve `-f` via `_read_filelist`, collect content |
| `resolve_canonical` | `(&Path) -> PathBuf` | Resolve canonical path for cache keys and cycle detection (canonicalize → resolve_absolute fallback → raw path fallback) |
| `process_content_items` | `(Vec<String>, check_exist, resolve_path, encode_with_env) -> (Vec<String>, Vec<String>)` | Post-processing pipeline |
| `parse_item_path` | `(&str) -> (&str, &str)` | Split `-v `/`-y `/`+incdir+` prefix from path |
| `resolve_absolute` | `(&str) -> Result<String, String>` | env_decode → join cwd → normalize `..` (no symlink resolve) |
| `encode_with_env_name` | `(&str, &str) -> Result<String, String>` | Replace path prefix matching `$ENV` value with `$ENV_NAME` |
| `dedup_vec` | `(&mut Vec<String>)` | In-place dedup, first-occurrence order |

### Parameter cascade (post-processing flags)

`encode_with_env` → `resolve_path=true` → `check_exist=true` → `deduplicate=true`

If none of the four flags is set, no dedup or post-processing occurs.

### Post-processing order

Skip `+libext+`/`+define+` etc. (opaque `+` directives, not `+incdir+`) → `parse_item_path` → `env_decode` → `resolve_absolute` → `Path::exists` (missing → error, kept in output) → `encode_with_env_name` → reconstruct item with prefix.

Re-deduplication happens after resolve/encode if `effective_dedup` is true (different relative paths may collapse to same absolute).

## CLI

```
flatten-filelist <filelist...> [--recursive] [--deduplicate] [--check-exist]
                 [--resolve-path] [--encode-with-env <NAME>] [-d DEFINE...]
```

Filelists must come first. Remaining flags are position-independent. `-d`/`--define` consumes all following args until the next flag. Default: non-recursive, no dedup, no post-processing. Content → stdout, errors → stderr (`ERROR: ` prefix). Exit 1 if errors.

## Python API

```python
flatten_filelist.flatten_filelist(
    filelist: str | list[str],
    directives: list[str],
    recursive: bool = False,
    deduplicate: bool = False,
    check_exist: bool = False,
    resolve_path: bool = False,
    encode_with_env: Optional[str] = None,
) -> tuple[list[str], list[str]]  # (content, errors)
```

Errors from stderr are stripped of `ERROR: ` prefix. Platform guard raises `RuntimeError` on non-Linux or non-x86_64.

## Gotchas

- **Empty/comment lines**: filtered at `_read_filelist_impl` level (empty or `//`), never reach output.
- **Circular includes**: detected via `visited: HashSet<PathBuf>` on call stack. Canonicalize → check visited → insert → recurse → remove.
- **Result caching**: `_read_filelist` caches parsed `(content, errors)` by canonical path in a `HashMap<PathBuf, (Vec<String>, Vec<String>)>`. Diamond includes (same file via different branches) hit the cache and are not re-parsed. Cache lives for the duration of one `read_filelist` call.
- **`` `define `` / `-def` accumulation**: ` `define `` lines skip the preprocessor-directive parser (line 177: `starts_with('`') && !starts_with("`define")`). They fall through to the active-block check: if active and not wildcard, the macro name is pushed into `directives` and the line is excluded from output. `-def` lines are handled the same way.
- **`directive_filter` mutates `directives`**: defines from nested filelists visible to later siblings.
- **`+libext+`/`+define+`**: opaque — no env expansion, no path resolution, no existence check.
- **Symlinks**: not resolved anywhere (use `normalize_path` not `canonicalize`), except cycle detection which uses `Path::canonicalize` for identity.
- **Platform**: musl x86_64 only; wheel tag is `py3-none-any` with runtime guard.
