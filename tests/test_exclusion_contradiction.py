from forkdrift import compare_artifacts, format_report, parse_artifact

LEFT_EXCLUDES = """interface shard-store

replicas = 3
excludes = legacy-mode
"""

RIGHT_DEFINES_TERM = """interface shard-store

replicas = 3
legacy-mode = on
"""

RIGHT_DEFINES_OPERATION = """interface shard-store

replicas = 3
operation legacy-mode = record
"""

RIGHT_NEUTRAL = """interface shard-store

replicas = 3
"""


def test_excluded_name_defined_as_term_alarms():
    left = parse_artifact(LEFT_EXCLUDES, "run-a")
    right = parse_artifact(RIGHT_DEFINES_TERM, "run-b")
    found = compare_artifacts(left, right)
    assert len(found) == 1
    d = found[0]
    assert d.name == "legacy-mode"
    assert d.kind == "excluded-but-defined"


def test_exclusion_alarm_quotes_both_lines_verbatim():
    left = parse_artifact(LEFT_EXCLUDES, "run-a")
    right = parse_artifact(RIGHT_DEFINES_TERM, "run-b")
    report = format_report(compare_artifacts(left, right))
    assert "excludes = legacy-mode" in report
    assert "legacy-mode = on" in report
    assert "run-a" in report
    assert "run-b" in report


def test_excluded_name_defined_as_operation_alarms():
    left = parse_artifact(LEFT_EXCLUDES, "run-a")
    right = parse_artifact(RIGHT_DEFINES_OPERATION, "run-b")
    found = compare_artifacts(left, right)
    assert len(found) == 1
    assert found[0].kind == "excluded-but-defined"


def test_excluded_name_not_defined_raises_no_alarm():
    left = parse_artifact(LEFT_EXCLUDES, "run-a")
    right = parse_artifact(RIGHT_NEUTRAL, "run-b")
    assert compare_artifacts(left, right) == []


def test_both_exclude_and_neither_defines_raises_no_alarm():
    left = parse_artifact(LEFT_EXCLUDES, "run-a")
    right = parse_artifact(LEFT_EXCLUDES, "run-b")
    assert compare_artifacts(left, right) == []


def test_exclusion_in_the_other_direction_alarms():
    left = parse_artifact(RIGHT_DEFINES_TERM, "run-a")
    right = parse_artifact(LEFT_EXCLUDES, "run-b")
    found = compare_artifacts(left, right)
    assert len(found) == 1
    d = found[0]
    assert d.kind == "excluded-but-defined"
    assert d.left_line == "legacy-mode = on"
    assert d.right_line == "excludes = legacy-mode"
