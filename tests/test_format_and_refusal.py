import pytest

from forkdrift import (
    ArtifactError,
    InterfaceMismatchError,
    compare_artifacts,
    parse_artifact,
)

VALID = """interface shard-store

replicas = 3
timeout = 30
operation fetch = record
label primary = node-a
excludes = legacy-mode, retired-flag
"""


def test_valid_artifact_parses_every_statement_form():
    art = parse_artifact(VALID, "run-a")
    assert art.interface == "shard-store"
    assert art.terms["replicas"].value == "3"
    assert art.terms["timeout"].value == "30"
    assert art.operations["fetch"].value == "record"
    assert art.labels["primary"].value == "node-a"
    assert art.excludes == frozenset({"legacy-mode", "retired-flag"})
    assert art.excludes_line == "excludes = legacy-mode, retired-flag"


def test_missing_interface_line_refused_by_name():
    with pytest.raises(ArtifactError) as excinfo:
        parse_artifact("replicas = 3\n", "run-a")
    assert "run-a" in str(excinfo.value)
    assert "interface" in str(excinfo.value)


def test_empty_artifact_refused_as_missing_interface():
    with pytest.raises(ArtifactError) as excinfo:
        parse_artifact("# only a comment\n\n", "run-a")
    assert "interface" in str(excinfo.value)


def test_line_without_equals_sign_refused_by_name():
    with pytest.raises(ArtifactError) as excinfo:
        parse_artifact("interface shard-store\n\nreplicas 3\n", "run-a")
    assert "run-a" in str(excinfo.value)
    assert "statement form" in str(excinfo.value)


def test_duplicate_term_refused_by_name():
    text = "interface shard-store\n\ntimeout = 30\ntimeout = 45\n"
    with pytest.raises(ArtifactError) as excinfo:
        parse_artifact(text, "run-b")
    assert "duplicate" in str(excinfo.value)
    assert "timeout" in str(excinfo.value)


def test_duplicate_operation_refused_by_name():
    text = (
        "interface shard-store\n\n"
        "operation fetch = record\noperation fetch = receipt\n"
    )
    with pytest.raises(ArtifactError) as excinfo:
        parse_artifact(text, "run-b")
    assert "duplicate" in str(excinfo.value)
    assert "fetch" in str(excinfo.value)


def test_duplicate_label_refused_by_name():
    text = (
        "interface shard-store\n\n"
        "label main = node-a\nlabel main = node-b\n"
    )
    with pytest.raises(ArtifactError) as excinfo:
        parse_artifact(text, "run-b")
    assert "duplicate" in str(excinfo.value)
    assert "main" in str(excinfo.value)


def test_empty_value_refused_by_name():
    text = "interface shard-store\n\ntimeout =\n"
    with pytest.raises(ArtifactError) as excinfo:
        parse_artifact(text, "run-b")
    assert "empty" in str(excinfo.value)
    assert "timeout" in str(excinfo.value)


def test_second_excludes_line_refused():
    text = "interface shard-store\n\nexcludes = a-mode\nexcludes = b-mode\n"
    with pytest.raises(ArtifactError):
        parse_artifact(text, "run-b")


def test_artifact_that_defines_what_it_excludes_refused():
    text = "interface shard-store\n\nlegacy-mode = on\nexcludes = legacy-mode\n"
    with pytest.raises(ArtifactError) as excinfo:
        parse_artifact(text, "run-b")
    assert "legacy-mode" in str(excinfo.value)
    assert "excludes" in str(excinfo.value)


def test_different_interfaces_refused_by_name():
    a = parse_artifact("interface shard-store\n\nreplicas = 3\n", "run-a")
    b = parse_artifact("interface shard-cache\n\nreplicas = 3\n", "run-b")
    with pytest.raises(InterfaceMismatchError) as excinfo:
        compare_artifacts(a, b)
    assert "shard-store" in str(excinfo.value)
    assert "shard-cache" in str(excinfo.value)
