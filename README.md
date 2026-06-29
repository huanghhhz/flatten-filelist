# flatten-filelist

Flatten Verilog filelists by resolving `-f` includes, expanding environment variables, and filtering preprocessor conditionals. Supports multiple top-level filelists.

## Install

```bash
pip install flatten-filelist
```

## Usage

```python
import flatten_filelist

# Single filelist, non-recursive (default)
content, errors = flatten_filelist.flatten_filelist(
    "filelist.f", ["DEFINE1", "DEFINE2"]
)

# Recursive -f resolution
content, errors = flatten_filelist.flatten_filelist(
    "filelist.f", ["DEFINE1"], recursive=True
)

# Multiple filelists
content, errors = flatten_filelist.flatten_filelist(
    ["filelist_a.f", "filelist_b.f"], ["DEFINE1"]
)

# Check existence, resolve to absolute paths
content, errors = flatten_filelist.flatten_filelist(
    "filelist.f", [], check_exist=True, resolve_path=True
)

# Encode absolute paths with an env var
content, errors = flatten_filelist.flatten_filelist(
    "filelist.f", [], encode_with_env="PROJ_HOME"
)
```

Returns `(content_lines, error_messages)` — errors are collected, not fatal.

### Parameters

| Param | Type | Default | Description |
|---|---|---|---|
| `filelist` | `str \| list[str]` | — | Path(s) to filelist file(s) |
| `directives` | `list[str]` | — | Preprocessor defines for conditional filtering |
| `recursive` | `bool` | `False` | Recursively resolve `-f` includes |
| `deduplicate` | `bool` | `False` | Deduplicate output lines |
| `check_exist` | `bool` | `False` | Check every path exists on disk |
| `resolve_path` | `bool` | `False` | Resolve to absolute paths (implies `check_exist`) |
| `encode_with_env` | `str \| None` | `None` | Encode paths with `$ENV_NAME` (implies `resolve_path`) |

**Cascade**: `encode_with_env` → `resolve_path` → `check_exist` → `deduplicate`.

## How it works

- Recursively follows `-f <path>` include directives (when `recursive=True`)
- Expands `$VAR` and `${VAR}` in paths using environment variables
- Evaluates Verilog preprocessor conditionals: `` `ifdef ``, `` `ifndef ``, `` `elsif ``, `` `else ``, `` `endif ``
- Accumulates `` `define `` and `-def` macros defined outside conditional blocks
- Supports wildcard mode (`"*"` directive includes all conditional branches)
- Omits empty lines and lines starting with `//` (comments)
- Optional deduplication (preserving first-occurrence order)
- Supports multiple top-level filelists (merged and deduplicated)
- Detects and reports circular `-f` includes instead of stack-overflowing
- Optional path existence checking, absolute path resolution, env-var encoding
- Handles `-v`, `-y`, `+incdir+` prefixes and opaque `+libext+`/`+define+` directives

## CLI Usage

```bash
# Non-recursive (default), no directives
flatten-filelist filelist.f

# Recursive -f resolution
flatten-filelist --recursive filelist.f -d DEFINE1 DEFINE2

# Multiple filelists
flatten-filelist --recursive a.f b.f -d DEFINE1 DEFINE2

# With post-processing
flatten-filelist --recursive --check-exist --resolve-path filelist.f
flatten-filelist --recursive --encode-with-env PROJ_HOME filelist.f
flatten-filelist --deduplicate filelist.f
```

```
Usage: flatten-filelist [--recursive] [--deduplicate] [--check-exist]
                        [--resolve-path] [--encode-with-env <NAME>]
                        <filelist...> [-d DEFINE...]
```

## Platform

Linux x86_64 only. Bundles a statically-linked musl binary that runs on any Linux kernel ≥2.6.
