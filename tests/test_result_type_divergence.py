from forkdrift import compare_artifacts, format_report, parse_artifact

LEFT = """interface shard-store

operation fetch = record
operation store = ack
"""

RIGHT = """interface shard-store

operation fetch = record
operation store = receipt
"""


def test_different_result_type_alarms_once():
    left = parse_artifact(LEFT, "run-a")
    right = parse_artifact(RIGHT, "run-b")
    found = compare_artifacts(left, right)
    assert len(found) == 1
    d = found[0]
    assert d.name == "store"
    assert d.kind == "result-type"


def test_result_type_alarm_quotes_both_operation_lines():
    left = parse_artifact(LEFT, "run-a")
    right = parse_artifact(RIGHT, "run-b")
    report = format_report(compare_artifacts(left, right))
    assert "operation store = ack" in report
    assert "operation store = receipt" in report
    assert "run-a" in report
    assert "run-b" in report


def test_same_result_type_raises_no_alarm():
    left = parse_artifact(LEFT, "run-a")
    right = parse_artifact(LEFT, "run-b")
    assert compare_artifacts(left, right) == []
