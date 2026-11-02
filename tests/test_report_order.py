from forkdrift import compare_artifacts, format_report, parse_artifact

LEFT = """interface shard-store

timeout = 30
backoff = fixed
"""

RIGHT = """interface shard-store

timeout = 45
backoff = exponential
"""


def test_two_divergences_are_both_reported_in_sorted_order():
    left = parse_artifact(LEFT, "run-a")
    right = parse_artifact(RIGHT, "run-b")
    found = compare_artifacts(left, right)
    assert [d.name for d in found] == ["backoff", "timeout"]
    report = format_report(found)
    assert report.index("backoff") < report.index("timeout")


def test_repeated_comparison_is_byte_identical():
    left = parse_artifact(LEFT, "run-a")
    right = parse_artifact(RIGHT, "run-b")
    first = format_report(compare_artifacts(left, right))
    second = format_report(compare_artifacts(left, right))
    assert first == second
