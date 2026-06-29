import os
import tempfile
from pathlib import Path

import flatten_filelist


def _write_filelist(path: Path, *lines: str) -> None:
    path.write_text("\n".join(lines) + "\n")


def test_single_file():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".f", delete=False) as f:
        _write_filelist(Path(f.name), "src/file1.v", "src/file2.v")

    try:
        content, errors = flatten_filelist.flatten_filelist(f.name, [])
        assert content == ["src/file1.v", "src/file2.v"]
        assert errors == []
    finally:
        os.unlink(f.name)


def test_nested_include():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".f", delete=False) as sub_f:
        _write_filelist(Path(sub_f.name), "sub/file.v")

    with tempfile.NamedTemporaryFile(mode="w", suffix=".f", delete=False) as main_f:
        _write_filelist(
            Path(main_f.name),
            "main/file1.v",
            f"-f {sub_f.name}",
            "main/file2.v",
        )

    try:
        content, errors = flatten_filelist.flatten_filelist(main_f.name, [])
        assert content == ["main/file1.v", "sub/file.v", "main/file2.v"]
        assert errors == []
    finally:
        os.unlink(sub_f.name)
        os.unlink(main_f.name)


def test_missing_file():
    content, errors = flatten_filelist.flatten_filelist("/nonexistent/filelist.f", [])
    assert content == []
    assert len(errors) == 1
    assert "not found" in errors[0]


def test_env_var_in_include():
    with tempfile.TemporaryDirectory() as tmpdir:
        sub_dir = Path(tmpdir) / "subdir"
        sub_dir.mkdir()
        sub_path = sub_dir / "sub.f"
        _write_filelist(sub_path, "sub/file.v")

        main_path = Path(tmpdir) / "main.f"
        _write_filelist(main_path, f"-f $TEST_FLAT_DIR/sub.f")

        os.environ["TEST_FLAT_DIR"] = str(sub_dir)

        try:
            content, errors = flatten_filelist.flatten_filelist(str(main_path), [])
            assert content == ["sub/file.v"]
            assert errors == []
        finally:
            del os.environ["TEST_FLAT_DIR"]


def test_missing_env_var_in_include():
    os.environ.pop("MISSING_FLAT_DIR", None)

    with tempfile.NamedTemporaryFile(mode="w", suffix=".f", delete=False) as f:
        _write_filelist(Path(f.name), "-f $MISSING_FLAT_DIR/filelist.f")

    try:
        content, errors = flatten_filelist.flatten_filelist(f.name, [])
        assert content == []
        assert len(errors) == 1
        assert "ENV_NOT_FOUND" in errors[0]
    finally:
        os.unlink(f.name)


def test_directive_filter_ifdef_matching():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".f", delete=False) as f:
        _write_filelist(
            Path(f.name),
            "file1.v",
            "`ifdef FOO",
            "file2.v",
            "`endif",
        )

    try:
        content, errors = flatten_filelist.flatten_filelist(f.name, ["FOO"])
        assert content == ["file1.v", "file2.v"]
        assert errors == []
    finally:
        os.unlink(f.name)


def test_directive_filter_ifdef_not_matching():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".f", delete=False) as f:
        _write_filelist(
            Path(f.name),
            "file1.v",
            "`ifdef FOO",
            "file2.v",
            "`endif",
        )

    try:
        content, errors = flatten_filelist.flatten_filelist(f.name, [])
        assert content == ["file1.v"]
        assert errors == []
    finally:
        os.unlink(f.name)


def test_directive_filter_ifndef():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".f", delete=False) as f:
        _write_filelist(
            Path(f.name),
            "`ifndef FOO",
            "file1.v",
            "`endif",
        )

    try:
        content, errors = flatten_filelist.flatten_filelist(f.name, [])
        assert content == ["file1.v"]
        assert errors == []
    finally:
        os.unlink(f.name)


def test_directive_filter_else():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".f", delete=False) as f:
        _write_filelist(
            Path(f.name),
            "`ifdef FOO",
            "file1.v",
            "`else",
            "file2.v",
            "`endif",
        )

    try:
        content, errors = flatten_filelist.flatten_filelist(f.name, [])
        assert content == ["file2.v"]
        assert errors == []
    finally:
        os.unlink(f.name)


def test_directive_filter_wildcard():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".f", delete=False) as f:
        _write_filelist(
            Path(f.name),
            "`ifdef FOO",
            "file1.v",
            "`endif",
        )

    try:
        content, errors = flatten_filelist.flatten_filelist(f.name, ["*"])
        assert content == ["file1.v"]
        assert errors == []
    finally:
        os.unlink(f.name)


def test_empty_filelist():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".f", delete=False) as f:
        pass

    try:
        content, errors = flatten_filelist.flatten_filelist(f.name, [])
        assert content == []
        assert errors == []
    finally:
        os.unlink(f.name)


def test_multiple_directives():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".f", delete=False) as f:
        _write_filelist(
            Path(f.name),
            "`ifdef A",
            "file_a.v",
            "`endif",
            "`ifdef B",
            "file_b.v",
            "`endif",
        )

    try:
        content, errors = flatten_filelist.flatten_filelist(f.name, ["A", "B"])
        assert content == ["file_a.v", "file_b.v"]
        assert errors == []
    finally:
        os.unlink(f.name)


def test_multiple_filelists():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".f", delete=False) as f1:
        _write_filelist(Path(f1.name), "a/file1.v", "common/dup.v")

    with tempfile.NamedTemporaryFile(mode="w", suffix=".f", delete=False) as f2:
        _write_filelist(Path(f2.name), "common/dup.v", "b/file2.v")

    try:
        content, errors = flatten_filelist.flatten_filelist(
            [f1.name, f2.name], []
        )
        assert content == ["a/file1.v", "common/dup.v", "b/file2.v"]
        assert errors == []
    finally:
        os.unlink(f1.name)
        os.unlink(f2.name)


def test_multiple_filelists_single_string():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".f", delete=False) as f:
        _write_filelist(Path(f.name), "src/file.v")

    try:
        content, errors = flatten_filelist.flatten_filelist(f.name, [])
        assert content == ["src/file.v"]
        assert errors == []
    finally:
        os.unlink(f.name)


def test_no_recursive():
    with tempfile.TemporaryDirectory() as tmpdir:
        sub_path = Path(tmpdir) / "sub.f"
        _write_filelist(sub_path, "sub/file.v")

        main_path = Path(tmpdir) / "main.f"
        _write_filelist(
            main_path,
            "main/file1.v",
            f"-f {sub_path}",
            "main/file2.v",
        )

        content, errors = flatten_filelist.flatten_filelist(
            str(main_path), [], no_recursive=True
        )
        assert content == [
            "main/file1.v",
            f"-f {sub_path}",
            "main/file2.v",
        ]
        assert errors == []
