import os
import subprocess
import sys
from pathlib import Path
from typing import List, Tuple

if sys.platform != "linux" or os.uname().machine != "x86_64":
    raise RuntimeError(
        "flatten-filelist only supports Linux x86_64"
    )


def _binary_path() -> Path:
    return Path(__file__).parent / "_bin" / "flatten-filelist"


def flatten_filelist(filelist: str, directives: List[str]) -> Tuple[List[str], List[str]]:
    """Flatten a Verilog filelist by resolving includes and filtering directives.

    Args:
        filelist: Path to the top-level filelist.
        directives: Preprocessor defines to use for conditional filtering.

    Returns:
        Tuple of (content_lines, error_messages).
    """
    bin_path = _binary_path()
    if not bin_path.is_file():
        return [], [f"binary not found: {bin_path}"]

    cmd = [str(bin_path), filelist, *directives]
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
