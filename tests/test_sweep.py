"""Pins for the sweep obligations FD-20 to FD-23.

The sweep compares one baseline artifact against each of several
others in turn. These pins hold the per-pair order of the library
entry point, the header line that attributes every pair in the
sweep document, the exit codes, and the rule that a refusal in any
pair refuses the whole sweep with no partial document printed.
"""

import os
import pathlib
import subprocess
import sys

import pytest

from forkdrift import (
    InterfaceMismatchError,
    compare_artifacts,
    format_report,
    format_sweep,
    parse_artifact,
    sweep_artifacts,
)

ROOT = pathlib.Path(__file__).resolve().parents[1]
SWEEP_COMMITTED = ROOT / "examples" / "sweep-divergences.txt"

BASELINE = "interface shard-store\n\ntimeout = 30\n"
DIVERGENT_TIMEOUT = "interface shard-store\n\ntimeout = 45\n"
DIVERGENT_STORE = "interface shard-store\n\ntimeout = 30\noperation store = ack\n"
MALFORMED = "timeout = 30\n"
OTHER_INTERFACE = "interface shard-cache\n\ntimeout = 30\n"


def artifact(text, source):
    return parse_artifact(text, source)


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


def test_sweep_returns_one_list_per_other_in_the_order_given():
    baseline = artifact(BASELINE, "base.tape")
    others = [
        artifact(DIVERGENT_TIMEOUT, "one.tape"),
        artifact(BASELINE, "two.tape"),
        artifact(DIVERGENT_STORE, "three.tape"),
    ]
    sweep = sweep_artifacts(baseline, others)
    assert len(sweep) == 3
    assert [d.name for d in sweep[0]] == ["timeout"]
    assert [d.kind for d in sweep[0]] == ["term-value"]
    assert sweep[1] == []
    assert [d.name for d in sweep[2]] == ["store"]
    assert [d.kind for d in sweep[2]] == ["presence"]


def test_sweep_lists_are_exactly_the_pairwise_comparisons():
    baseline = artifact(BASELINE, "base.tape")
    others = [
        artifact(DIVERGENT_TIMEOUT, "one.tape"),
        artifact(DIVERGENT_STORE, "two.tape"),
    ]
    sweep = sweep_artifacts(baseline, others)
    for other, divergences in zip(others, sweep):
        assert divergences == compare_artifacts(baseline, other)


def test_sweep_applies_the_interface_refusal_pair_by_pair():
    baseline = artifact(BASELINE, "base.tape")
    others = [
        artifact(BASELINE, "one.tape"),
        artifact(OTHER_INTERFACE, "two.tape"),
    ]
    with pytest.raises(InterfaceMismatchError) as excinfo:
        sweep_artifacts(baseline, others)
    assert "shard-store" in str(excinfo.value)
    assert "shard-cache" in str(excinfo.value)


def test_format_sweep_attributes_an_agreeing_pair_with_its_header_alone():
    baseline = artifact(BASELINE, "base.tape")
    others = [
        artifact(DIVERGENT_TIMEOUT, "one.tape"),
        artifact(BASELINE, "two.tape"),
    ]
    document = format_sweep(baseline, others, sweep_artifacts(baseline, others))
    expected = (
        "pair left (base.tape) right (one.tape)\n"
        + format_report(compare_artifacts(baseline, others[0]))
        + "pair left (base.tape) right (two.tape)\n"
    )
    assert document == expected


def test_format_sweep_all_agreeing_is_exactly_the_header_lines():
    baseline = artifact(BASELINE, "base.tape")
    others = [artifact(BASELINE, "one.tape"), artifact(BASELINE, "two.tape")]
    document = format_sweep(baseline, others, sweep_artifacts(baseline, others))
    assert document == (
        "pair left (base.tape) right (one.tape)\n"
        "pair left (base.tape) right (two.tape)\n"
    )


def test_cli_sweep_all_agreeing_exits_0_with_only_the_header_lines(tmp_path):
    baseline = write(tmp_path, "base.tape", BASELINE)
    one = write(tmp_path, "one.tape", BASELINE)
    two = write(tmp_path, "two.tape", BASELINE)
    result = run_cli("sweep", baseline, one, two)
    assert result.returncode == 0
    assert result.stdout == (
        f"pair left ({baseline}) right ({one})\n"
        f"pair left ({baseline}) right ({two})\n"
    )
    assert result.stderr == ""


def test_cli_sweep_with_a_divergence_exits_1_and_attributes_the_pair(tmp_path):
    baseline = write(tmp_path, "base.tape", BASELINE)
    agreeing = write(tmp_path, "one.tape", BASELINE)
    divergent = write(tmp_path, "two.tape", DIVERGENT_TIMEOUT)
    result = run_cli("sweep", baseline, agreeing, divergent)
    assert result.returncode == 1
    assert result.stdout == (
        f"pair left ({baseline}) right ({agreeing})\n"
        f"pair left ({baseline}) right ({divergent})\n"
        "divergence shard-store timeout term-value\n"
        f"  left ({baseline}) timeout = 30\n"
        f"  right ({divergent}) timeout = 45\n"
    )


def test_cli_sweep_refusal_in_a_late_pair_prints_no_partial_document(tmp_path):
    baseline = write(tmp_path, "base.tape", BASELINE)
    divergent = write(tmp_path, "one.tape", DIVERGENT_TIMEOUT)
    malformed = write(tmp_path, "two.tape", MALFORMED)
    result = run_cli("sweep", baseline, divergent, malformed)
    assert result.returncode == 2
    assert result.stdout == ""
    assert result.stderr.startswith("error:")
    assert result.stderr.count("\n") == 1
    assert "Traceback" not in result.stderr


def test_cli_sweep_missing_file_exits_2_with_no_document(tmp_path):
    baseline = write(tmp_path, "base.tape", BASELINE)
    missing = str(tmp_path / "nope.tape")
    result = run_cli("sweep", baseline, missing)
    assert result.returncode == 2
    assert result.stdout == ""
    assert result.stderr.startswith("error:")
    assert "cannot read" in result.stderr
    assert "Traceback" not in result.stderr


def test_cli_sweep_interface_mismatch_in_a_later_pair_refuses_the_sweep(tmp_path):
    baseline = write(tmp_path, "base.tape", BASELINE)
    agreeing = write(tmp_path, "one.tape", BASELINE)
    other = write(tmp_path, "two.tape", OTHER_INTERFACE)
    result = run_cli("sweep", baseline, agreeing, other)
    assert result.returncode == 2
    assert result.stdout == ""
    assert "different interfaces" in result.stderr
    assert "Traceback" not in result.stderr


def test_cli_sweep_reports_the_first_refusal_in_pair_order(tmp_path):
    baseline = write(tmp_path, "base.tape", BASELINE)
    mismatched = write(tmp_path, "one.tape", OTHER_INTERFACE)
    malformed = write(tmp_path, "two.tape", MALFORMED)
    first = run_cli("sweep", baseline, mismatched, malformed)
    assert first.returncode == 2
    assert "different interfaces" in first.stderr
    second = run_cli("sweep", baseline, malformed, mismatched)
    assert second.returncode == 2
    assert "different interfaces" not in second.stderr
    assert "malformed" in second.stderr


def test_cli_sweep_with_no_other_artifact_is_a_usage_error(tmp_path):
    baseline = write(tmp_path, "base.tape", BASELINE)
    result = run_cli("sweep", baseline)
    assert result.returncode == 2
    assert result.stdout == ""
    assert "usage: forkdrift sweep" in result.stderr


def run_example_sweep():
    env = dict(os.environ)
    env["PYTHONPATH"] = str(ROOT)
    return subprocess.run(
        [sys.executable, "-m", "forkdrift", "sweep",
         "examples/run-a.tape", "examples/run-b.tape",
         "examples/presence-a.tape", "examples/presence-b.tape"],
        cwd=ROOT,
        env=env,
        capture_output=True,
    )


def test_committed_sweep_example_matches_regeneration():
    result = run_example_sweep()
    assert result.returncode == 1
    assert result.stdout == SWEEP_COMMITTED.read_bytes()
    second = run_example_sweep()
    assert second.stdout == result.stdout


def test_committed_sweep_example_attributes_all_three_pairs():
    text = SWEEP_COMMITTED.read_text()
    headers = [ln for ln in text.splitlines() if ln.startswith("pair left")]
    assert headers == [
        "pair left (examples/run-a.tape) right (examples/run-b.tape)",
        "pair left (examples/run-a.tape) right (examples/presence-a.tape)",
        "pair left (examples/run-a.tape) right (examples/presence-b.tape)",
    ]
