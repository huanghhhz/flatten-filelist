# flatten-filelist

Flatten Verilog filelists by resolving `-f` includes, expanding environment variables, and filtering preprocessor conditionals.

## Install

```bash
pip install flatten-filelist
```

## Usage

```python
import flatten_filelist

content, errors = flatten_filelist.flatten_filelist("filelist.f", ["DEFINE1", "DEFINE2"])
```

Returns `(content_lines, error_messages)` — errors are collected, not fatal.

## How it works

- Recursively follows `-f <path>` include directives
- Expands `$VAR` and `${VAR}` in paths using environment variables
- Evaluates Verilog preprocessor conditionals: `` `ifdef ``, `` `ifndef ``, `` `elsif ``, `` `else ``, `` `endif ``
- Accumulates `` `define `` and `-def` macros encountered in active conditional branches
- Supports wildcard mode (`"*"` directive includes all conditional branches)

## Platform

Linux x86_64 only. Bundles a statically-linked musl binary that runs on any Linux kernel ≥2.6.
