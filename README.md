# flatten-filelist

Flatten Verilog filelists by resolving `-f` includes, expanding environment variables, and filtering preprocessor conditionals. Supports multiple top-level filelists.

## Install

```bash
pip install flatten-filelist
```

## Usage

```python
import flatten_filelist

# Single filelist
content, errors = flatten_filelist.flatten_filelist("filelist.f", ["DEFINE1", "DEFINE2"])

# Multiple filelists
content, errors = flatten_filelist.flatten_filelist(
    ["filelist_a.f", "filelist_b.f"], ["DEFINE1"]
)

# Skip recursive -f resolution
content, errors = flatten_filelist.flatten_filelist(
    "filelist.f", [], no_recursive=True
)
```

Returns `(content_lines, error_messages)` — errors are collected, not fatal.

## How it works

- Recursively follows `-f <path>` include directives
- Expands `$VAR` and `${VAR}` in paths using environment variables
- Evaluates Verilog preprocessor conditionals: `` `ifdef ``, `` `ifndef ``, `` `elsif ``, `` `else ``, `` `endif ``
- Accumulates `` `define `` and `-def` macros defined outside conditional blocks
- Supports wildcard mode (`"*"` directive includes all conditional branches)
- Omits empty lines and lines starting with `//` (comments)
- Deduplicates output (preserving first-occurrence order)
- Supports multiple top-level filelists (merged and deduplicated)

## CLI Usage

```bash
# Single filelist
flatten-filelist filelist.f -d DEFINE1 DEFINE2

# Multiple filelists
flatten-filelist a.f b.f -d DEFINE1 DEFINE2

# Skip recursive -f resolution
flatten-filelist --no-recursive filelist.f

# No directives (just flatten)
flatten-filelist filelist.f
```

## Platform

Linux x86_64 only. Bundles a statically-linked musl binary that runs on any Linux kernel ≥2.6.
