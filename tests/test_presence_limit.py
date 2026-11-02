"""FD-16 pins, the interplay of presence and the exclusion rules.

Round 1 pinned the FD-8 presence limit in this file. FD-15 retired
that limit, so the two pins that asserted silence for one-sided
statements were superseded by the retirement itself and are replaced
by the presence pins in test_presence.py. What survives of the old
limit is the exclusion interplay, and it lives here.
"""

from forkdrift import compare_artifacts, parse_artifact

DEFINING = """interface shard-store

replicas = 3
timeout = 30
"""

DUAL_DEFINING = """interface shard-store

replicas = 3
timeout = 30
operation timeout = ack
"""

EXCLUDING = """interface shard-store

replicas = 3
excludes = timeout
"""

BARE = """interface shard-store

replicas = 3
"""


def test_excluded_one_sided_name_raises_only_the_exclusion_contradiction():
    left = parse_artifact(DEFINING, "run-a")
    right = parse_artifact(EXCLUDING, "run-b")
    found = compare_artifacts(left, right)
    assert len(found) == 1
    assert found[0].name == "timeout"
    assert found[0].kind == "excluded-but-defined"


def test_exclusion_contradiction_fires_in_the_other_direction_too():
    left = parse_artifact(EXCLUDING, "run-a")
    right = parse_artifact(DEFINING, "run-b")
    found = compare_artifacts(left, right)
    assert len(found) == 1
    assert found[0].name == "timeout"
    assert found[0].kind == "excluded-but-defined"


def test_excluded_name_defined_both_ways_still_raises_one_divergence():
    left = parse_artifact(DUAL_DEFINING, "run-a")
    right = parse_artifact(EXCLUDING, "run-b")
    found = compare_artifacts(left, right)
    assert len(found) == 1
    assert found[0].name == "timeout"
    assert found[0].kind == "excluded-but-defined"


def test_excluded_name_defined_by_neither_side_raises_nothing():
    left = parse_artifact(EXCLUDING, "run-a")
    right = parse_artifact(BARE, "run-b")
    assert compare_artifacts(left, right) == []


def test_name_excluded_by_both_sides_raises_nothing():
    left = parse_artifact(EXCLUDING, "run-a")
    right = parse_artifact(EXCLUDING, "run-b")
    assert compare_artifacts(left, right) == []
