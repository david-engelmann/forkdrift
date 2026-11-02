from forkdrift import compare_artifacts, parse_artifact

BASE = """interface shard-store

replicas = 3
timeout = 30
operation fetch = list of records
label primary = node-a
"""

REORDERED = """interface shard-store

label primary = node-a
operation fetch = list of records
timeout = 30
replicas = 3
"""

WHITESPACE = """interface shard-store

replicas   =   3
timeout=30
operation fetch = list  of   records
label primary = node-a
"""

NUMERIC_FORMAT = """interface shard-store

replicas = 3.00
timeout = 30.0
operation fetch = list of records
label primary = node-a
"""

LABEL_RENAMED = """interface shard-store

replicas = 3
timeout = 30
operation fetch = list of records
label main = node-a
"""

LABEL_REVALUED = """interface shard-store

replicas = 3
timeout = 30
operation fetch = list of records
label primary = node-b
"""

NUMERIC_WORTH_DIFFERS = """interface shard-store

replicas = 3
timeout = 31
operation fetch = list of records
label primary = node-a
"""

WORD_DIFFERS = """interface shard-store

replicas = 3
timeout = 30
operation fetch = list of record
label primary = node-a
"""


def assert_agree(variant):
    left = parse_artifact(BASE, "run-a")
    right = parse_artifact(variant, "run-b")
    assert compare_artifacts(left, right) == []


def test_reordered_lines_raise_no_alarm():
    assert_agree(REORDERED)


def test_whitespace_differences_raise_no_alarm():
    assert_agree(WHITESPACE)


def test_numeric_formatting_raises_no_alarm():
    assert_agree(NUMERIC_FORMAT)


def test_renamed_label_raises_no_alarm():
    assert_agree(LABEL_RENAMED)


def test_revalued_label_raises_no_alarm():
    assert_agree(LABEL_REVALUED)


def test_numeric_worth_difference_still_alarms():
    left = parse_artifact(BASE, "run-a")
    right = parse_artifact(NUMERIC_WORTH_DIFFERS, "run-b")
    found = compare_artifacts(left, right)
    assert len(found) == 1
    assert found[0].name == "timeout"
    assert found[0].kind == "term-value"


def test_one_word_difference_in_a_type_still_alarms():
    left = parse_artifact(BASE, "run-a")
    right = parse_artifact(WORD_DIFFERS, "run-b")
    found = compare_artifacts(left, right)
    assert len(found) == 1
    assert found[0].name == "fetch"
    assert found[0].kind == "result-type"
