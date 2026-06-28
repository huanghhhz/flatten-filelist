import os
import subprocess
import sys
from pathlib import Path
from typing import List, Optional, Tuple, Union


if sys.platform != "linux" or os.uname().machine != "x86_64":
    raise RuntimeError(
        "flatten-filelist only supports Linux x86_64"
    )


def _binary_path() -> Path:
    return Path(__file__).parent / "_bin" / "flatten-filelist"


def flatten_filelist(
    filelist: Union[str, List[str]],
    directives: List[str],
    no_recursive: bool = False,
    deduplication: bool = False,
    check_exist: bool = False,
    resolve_path: bool = False,
    encode_with_env: Optional[str] = None,
) -> Tuple[List[str], List[str]]:
    """Flatten Verilog filelists by resolving includes and filtering directives.

    Args:
        filelist: Path to a single filelist, or a list of filelist paths.
        directives: Preprocessor defines to use for conditional filtering.
        no_recursive: If True, do not recursively resolve -f include directives.
        deduplication: If True, deduplicate output lines. Implied by check_exist,
            resolve_path, or encode_with_env.
        check_exist: If True, check that every output item exists on disk.
            Handles -v/-y/+incdir+ prefixes and env vars in paths.
        resolve_path: If True, resolve every output item to an absolute path
            (expands env vars, resolves ..).  Implies check_exist=True.
        encode_with_env: If set, resolve paths (as above) and then replace the
            matching prefix with $ENV_NAME.  Implies resolve_path=True.

    Returns:
        Tuple of (content_lines, error_messages).
    """
    bin_path = _binary_path()
    if not bin_path.is_file():
        return [], [f"binary not found: {bin_path}"]

    filelists = [filelist] if isinstance(filelist, str) else list(filelist)
    cmd = [str(bin_path)]
    if no_recursive:
        cmd.append("--no-recursive")
    if deduplication:
        cmd.append("--deduplication")
    if resolve_path:
        cmd.append("--resolve-path")
    if encode_with_env is not None:
        cmd.extend(["--encode-with-env", encode_with_env])
    cmd.extend(filelists)
    if directives:
        cmd.append("-d")
        cmd.extend(directives)

    try:
        proc = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
        )
    except OSError as e:
        return [], [f"failed to run {bin_path}: {e}"]

    stdout_lines = proc.stdout.splitlines()
    stderr_lines = proc.stderr.splitlines()

    errors = []
    for line in stderr_lines:
        if line.startswith("ERROR: "):
            errors.append(line[7:])
        elif line.strip():
            errors.append(line)

    return stdout_lines, errors
