import os
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]

AGREEING = "interface shard-store\n\ntimeout = 30\n"
DIVERGENT = "interface shard-store\n\ntimeout = 45\n"
MALFORMED = "timeout = 30\n"
OTHER_INTERFACE = "interface shard-cache\n\ntimeout = 30\n"


def run_cli(*args):
    env = dict(os.environ)
    env["PYTHONPATH"] = str(ROOT)
    return subprocess.run(
        [sys.executable, "-m", "forkdrift", *args],
        cwd=ROOT,
        env=env,
        capture_output=True,
        text=True,
    )


def write(tmp_path, name, text):
    path = tmp_path / name
    path.write_text(text)
    return str(path)


def test_cli_agreeing_pair_exits_0_with_empty_output(tmp_path):
    left = write(tmp_path, "a.tape", AGREEING)
    right = write(tmp_path, "b.tape", AGREEING)
    result = run_cli("compare", left, right)
    assert result.returncode == 0
    assert result.stdout == ""


def test_cli_divergent_pair_exits_1_and_quotes_both_definitions(tmp_path):
    left = write(tmp_path, "a.tape", AGREEING)
    right = write(tmp_path, "b.tape", DIVERGENT)
    result = run_cli("compare", left, right)
    assert result.returncode == 1
    assert "timeout = 30" in result.stdout
    assert "timeout = 45" in result.stdout


def test_cli_malformed_input_exits_2_without_traceback(tmp_path):
    left = write(tmp_path, "a.tape", MALFORMED)
    right = write(tmp_path, "b.tape", AGREEING)
    result = run_cli("compare", left, right)
    assert result.returncode == 2
    assert result.stderr.startswith("error:")
    assert "interface" in result.stderr
    assert "Traceback" not in result.stderr


def test_cli_missing_file_exits_2_without_traceback(tmp_path):
    missing = str(tmp_path / "nope.tape")
    right = write(tmp_path, "b.tape", AGREEING)
    result = run_cli("compare", missing, right)
    assert result.returncode == 2
    assert "Traceback" not in result.stderr


def test_cli_different_interfaces_exit_2(tmp_path):
    left = write(tmp_path, "a.tape", AGREEING)
    right = write(tmp_path, "b.tape", OTHER_INTERFACE)
    result = run_cli("compare", left, right)
    assert result.returncode == 2
    assert "Traceback" not in result.stderr
