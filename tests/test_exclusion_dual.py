"""FD-12 and FD-14 regression pins from the Round 1 review fixes."""

from forkdrift.artifacts import parse_artifact
from forkdrift.divergence import EXCLUDED_BUT_DEFINED, compare_artifacts, format_report


def artifact(text, source):
    return parse_artifact(text, source)


def test_excluded_name_defined_as_term_and_operation_is_one_divergence():
    left = artifact("interface shard-store\nexcludes = dual\n", "a.tape")
    right = artifact(
        "interface shard-store\ndual = 1\noperation dual = ack\n", "b.tape"
    )
    found = compare_artifacts(left, right)
    assert len(found) == 1
    only = found[0]
    assert only.kind == EXCLUDED_BUT_DEFINED and only.name == "dual"
    report = format_report(found)
    assert "excludes = dual" in report
    assert "dual = 1" in report
    assert "operation dual = ack" in report


def test_excluded_name_defined_as_term_and_operation_reverse_direction():
    left = artifact(
        "interface shard-store\ndual = 1\noperation dual = ack\n", "a.tape"
    )
    right = artifact("interface shard-store\nexcludes = dual\n", "b.tape")
    found = compare_artifacts(left, right)
    assert len(found) == 1
    report = format_report(found)
    assert "dual = 1" in report and "operation dual = ack" in report


def test_label_only_name_may_be_excluded_by_its_own_artifact():
    parsed = artifact(
        "interface shard-store\nlabel legacy = node-a\nexcludes = legacy\n",
        "a.tape",
    )
    assert "legacy" in parsed.labels
