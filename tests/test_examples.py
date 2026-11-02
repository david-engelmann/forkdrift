import os
import pathlib
import subprocess
import sys

from forkdrift import compare_artifacts, parse_artifact

ROOT = pathlib.Path(__file__).resolve().parents[1]
LEFT = ROOT / "examples" / "run-a.tape"
RIGHT = ROOT / "examples" / "run-b.tape"
COMMITTED = ROOT / "examples" / "divergences.txt"
PRESENCE_LEFT = ROOT / "examples" / "presence-a.tape"
PRESENCE_RIGHT = ROOT / "examples" / "presence-b.tape"
PRESENCE_COMMITTED = ROOT / "examples" / "presence-divergences.txt"


def run_example_compare():
    env = dict(os.environ)
    env["PYTHONPATH"] = str(ROOT)
    return subprocess.run(
        [sys.executable, "-m", "forkdrift", "compare",
         "examples/run-a.tape", "examples/run-b.tape"],
        cwd=ROOT,
        env=env,
        capture_output=True,
    )


def run_presence_example_compare():
    env = dict(os.environ)
    env["PYTHONPATH"] = str(ROOT)
    return subprocess.run(
        [sys.executable, "-m", "forkdrift", "compare",
         "examples/presence-a.tape", "examples/presence-b.tape"],
        cwd=ROOT,
        env=env,
        capture_output=True,
    )


def test_example_pair_carries_exactly_two_divergences():
    left = parse_artifact(LEFT.read_text(), "examples/run-a.tape")
    right = parse_artifact(RIGHT.read_text(), "examples/run-b.tape")
    found = compare_artifacts(left, right)
    assert [d.name for d in found] == ["store", "timeout"]
    assert [d.kind for d in found] == ["result-type", "term-value"]


def test_committed_example_output_matches_regeneration():
    result = run_example_compare()
    assert result.returncode == 1
    assert result.stdout == COMMITTED.read_bytes()
    second = run_example_compare()
    assert second.stdout == result.stdout


def test_presence_example_pair_carries_a_mixed_report_in_order():
    left = parse_artifact(PRESENCE_LEFT.read_text(), "examples/presence-a.tape")
    right = parse_artifact(
        PRESENCE_RIGHT.read_text(), "examples/presence-b.tape"
    )
    found = compare_artifacts(left, right)
    assert [d.name for d in found] == ["purge", "replicas"]
    assert [d.kind for d in found] == ["presence", "term-value"]


def test_committed_presence_example_output_matches_regeneration():
    result = run_presence_example_compare()
    assert result.returncode == 1
    assert result.stdout == PRESENCE_COMMITTED.read_bytes()
    second = run_presence_example_compare()
    assert second.stdout == result.stdout
