from forkdrift import compare_artifacts, format_report, parse_artifact

LEFT = """interface shard-store

replicas = 3
timeout = 30
"""

RIGHT = """interface shard-store

replicas = 3
timeout = 45
"""


def test_incompatible_term_value_alarms_once():
    left = parse_artifact(LEFT, "run-a")
    right = parse_artifact(RIGHT, "run-b")
    found = compare_artifacts(left, right)
    assert len(found) == 1
    assert found[0].kind == "term-value"


def test_alarm_quotes_both_definitions_verbatim_with_sources():
    left = parse_artifact(LEFT, "run-a")
    right = parse_artifact(RIGHT, "run-b")
    report = format_report(compare_artifacts(left, right))
    assert "timeout = 30" in report
    assert "timeout = 45" in report
    assert "run-a" in report
    assert "run-b" in report


def test_divergence_names_the_concept_and_both_runs():
    left = parse_artifact(LEFT, "run-a")
    right = parse_artifact(RIGHT, "run-b")
    d = compare_artifacts(left, right)[0]
    assert d.interface == "shard-store"
    assert d.name == "timeout"
    assert d.left_source == "run-a"
    assert d.right_source == "run-b"
    assert d.left_line == "timeout = 30"
    assert d.right_line == "timeout = 45"


def test_agreeing_terms_raise_no_alarm():
    left = parse_artifact(LEFT, "run-a")
    right = parse_artifact(LEFT, "run-b")
    assert compare_artifacts(left, right) == []
