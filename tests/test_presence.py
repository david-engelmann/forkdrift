"""FD-15 and FD-17 pins, the presence kind and its report order."""

import os
import pathlib
import subprocess
import sys

from forkdrift import compare_artifacts, format_report, parse_artifact

ROOT = pathlib.Path(__file__).resolve().parents[1]

TERM_LEFT = """interface shard-store

replicas = 3
timeout = 30
"""

TERM_RIGHT = """interface shard-store

replicas = 3
"""

OPERATION_RIGHT = """interface shard-store

replicas = 3
operation purge = ack
"""

DUAL_LEFT = """interface shard-store

timeout = 30
operation timeout = ack
"""

EMPTY = """interface shard-store
"""

CROSS_LEFT = """interface shard-store

cache = on
"""

CROSS_RIGHT = """interface shard-store

operation cache = record
"""


def test_one_sided_term_raises_one_presence_divergence():
    left = parse_artifact(TERM_LEFT, "run-a")
    right = parse_artifact(TERM_RIGHT, "run-b")
    found = compare_artifacts(left, right)
    assert len(found) == 1
    only = found[0]
    assert only.name == "timeout"
    assert only.kind == "presence"
    assert only.left_source == "run-a"
    assert only.left_line == "timeout = 30"
    assert only.right_source == "run-b"
    assert only.right_line == "interface shard-store"
    assert format_report(found) == (
        "divergence shard-store timeout presence\n"
        "  left (run-a) timeout = 30\n"
        "  right (run-b) interface shard-store\n"
    )


def test_one_sided_operation_raises_one_presence_divergence():
    left = parse_artifact(TERM_RIGHT, "run-a")
    right = parse_artifact(OPERATION_RIGHT, "run-b")
    found = compare_artifacts(left, right)
    assert len(found) == 1
    only = found[0]
    assert only.name == "purge"
    assert only.kind == "presence"
    assert only.left_line == "interface shard-store"
    assert only.right_source == "run-b"
    assert only.right_line == "operation purge = ack"
    assert format_report(found) == (
        "divergence shard-store purge presence\n"
        "  left (run-a) interface shard-store\n"
        "  right (run-b) operation purge = ack\n"
    )


def test_name_defined_both_ways_on_one_side_raises_one_presence_block():
    left = parse_artifact(DUAL_LEFT, "run-a")
    right = parse_artifact(EMPTY, "run-b")
    found = compare_artifacts(left, right)
    assert len(found) == 1
    assert found[0].name == "timeout"
    assert found[0].kind == "presence"
    assert format_report(found) == (
        "divergence shard-store timeout presence\n"
        "  left (run-a) timeout = 30\n"
        "  right (run-b) interface shard-store\n"
        "  right (run-a) operation timeout = ack\n"
    )


def test_term_on_one_side_and_operation_on_the_other_raise_one_block():
    left = parse_artifact(CROSS_LEFT, "run-a")
    right = parse_artifact(CROSS_RIGHT, "run-b")
    found = compare_artifacts(left, right)
    assert len(found) == 1
    only = found[0]
    assert only.name == "cache"
    assert only.kind == "presence"
    assert only.left_line == "cache = on"
    assert only.right_line == "operation cache = record"
    assert format_report(found) == (
        "divergence shard-store cache presence\n"
        "  left (run-a) cache = on\n"
        "  right (run-b) operation cache = record\n"
    )


def test_shared_term_with_different_values_raises_no_presence():
    left = parse_artifact(TERM_LEFT, "run-a")
    right = parse_artifact(
        "interface shard-store\n\nreplicas = 3\ntimeout = 45\n", "run-b"
    )
    found = compare_artifacts(left, right)
    assert [d.kind for d in found] == ["term-value"]


def test_shared_operation_with_different_types_raises_no_presence():
    left = parse_artifact(OPERATION_RIGHT, "run-a")
    right = parse_artifact(
        "interface shard-store\n\nreplicas = 3\noperation purge = receipt\n",
        "run-b",
    )
    found = compare_artifacts(left, right)
    assert [d.kind for d in found] == ["result-type"]


def test_shared_form_differing_only_by_normalization_raises_nothing():
    left = parse_artifact(
        "interface shard-store\n\ntimeout = 030\nnote = a  b\n", "run-a"
    )
    right = parse_artifact(
        "interface shard-store\n\ntimeout = 30\nnote = a b\n", "run-b"
    )
    assert compare_artifacts(left, right) == []


def test_one_sided_operation_beside_a_shared_term_raises_presence_only():
    left = parse_artifact(
        "interface shard-store\n\n"
        "replicas = 3\ntimeout = 30\noperation purge = ack\n",
        "run-a",
    )
    right = parse_artifact(TERM_LEFT, "run-b")
    found = compare_artifacts(left, right)
    assert len(found) == 1
    only = found[0]
    assert only.name == "purge"
    assert only.kind == "presence"
    assert only.left_line == "operation purge = ack"
    assert only.right_line == "interface shard-store"


def test_diverging_term_and_one_sided_operation_raise_both_kinds():
    left = parse_artifact(
        "interface shard-store\n\nmode = fast\noperation mode = ack\n",
        "run-a",
    )
    right = parse_artifact("interface shard-store\n\nmode = slow\n", "run-b")
    found = compare_artifacts(left, right)
    assert [d.name for d in found] == ["mode", "mode"]
    assert [d.kind for d in found] == ["presence", "term-value"]
    presence = found[0]
    assert presence.left_line == "operation mode = ack"
    assert presence.right_line == "interface shard-store"


def test_label_stated_on_one_side_only_raises_nothing():
    left = parse_artifact(
        "interface shard-store\n\nreplicas = 3\nlabel primary = node-a\n",
        "run-a",
    )
    right = parse_artifact(TERM_RIGHT, "run-b")
    assert compare_artifacts(left, right) == []


def test_presence_and_value_divergences_sort_by_name_then_kind():
    left = parse_artifact(
        "interface shard-store\n\n"
        "replicas = 3\ntimeout = 30\noperation purge = ack\n",
        "run-a",
    )
    right = parse_artifact(
        "interface shard-store\n\nreplicas = 4\ntimeout = 30\n", "run-b"
    )
    found = compare_artifacts(left, right)
    assert format_report(found) == (
        "divergence shard-store purge presence\n"
        "  left (run-a) operation purge = ack\n"
        "  right (run-b) interface shard-store\n"
        "divergence shard-store replicas term-value\n"
        "  left (run-a) replicas = 3\n"
        "  right (run-b) replicas = 4\n"
    )


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


def test_cli_pair_with_only_a_presence_divergence_exits_1(tmp_path):
    left = tmp_path / "a.tape"
    left.write_text(TERM_LEFT)
    right = tmp_path / "b.tape"
    right.write_text(TERM_RIGHT)
    result = run_cli("compare", str(left), str(right))
    assert result.returncode == 1
    assert "divergence shard-store timeout presence" in result.stdout
    assert result.stderr == ""
