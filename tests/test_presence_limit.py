from forkdrift import compare_artifacts, parse_artifact

FULL = """interface shard-store

replicas = 3
timeout = 30
operation purge = ack
"""

SPARSE = """interface shard-store

replicas = 3
"""

EXCLUDING = """interface shard-store

replicas = 3
excludes = timeout
"""


def test_term_stated_on_one_side_only_raises_no_alarm():
    left = parse_artifact(FULL, "run-a")
    right = parse_artifact(SPARSE, "run-b")
    assert compare_artifacts(left, right) == []


def test_operation_stated_on_one_side_only_raises_no_alarm():
    left = parse_artifact(SPARSE, "run-a")
    right = parse_artifact(FULL, "run-b")
    assert compare_artifacts(left, right) == []


def test_one_sided_term_excluded_by_the_other_side_alarms():
    left = parse_artifact(FULL, "run-a")
    right = parse_artifact(EXCLUDING, "run-b")
    found = compare_artifacts(left, right)
    assert len(found) == 1
    assert found[0].name == "timeout"
    assert found[0].kind == "excluded-but-defined"
