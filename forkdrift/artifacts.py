"""Parsing for recorded contract artifacts.

An artifact is the recorded text one run left behind for one
interface, written in the line based format CONTRACT.md defines.
"""

import re
from dataclasses import dataclass
from decimal import Decimal, InvalidOperation

from .errors import ArtifactError

IDENTIFIER = re.compile(r"[A-Za-z_][A-Za-z0-9_.\-]*")


@dataclass(frozen=True)
class Statement:
    name: str
    value: str
    line: str


@dataclass(frozen=True)
class Artifact:
    source: str
    interface: str
    interface_line: str
    terms: dict
    operations: dict
    labels: dict
    excludes: frozenset
    excludes_line: str


def normalize_value(value):
    """Collapse every run of whitespace in a value to a single space."""
    return " ".join(value.split())


def values_equal(left, right):
    """Compare two normalized values under the FD-6 normalizations."""
    if left == right:
        return True
    try:
        left_number = Decimal(left)
        right_number = Decimal(right)
    except InvalidOperation:
        return False
    if not (left_number.is_finite() and right_number.is_finite()):
        return False
    return left_number == right_number


def parse_artifact(text, source):
    """Parse one recorded artifact or raise ArtifactError naming why."""
    meaningful = []
    for lineno, raw in enumerate(text.splitlines(), start=1):
        stripped = raw.strip()
        if not stripped or stripped.startswith("#"):
            continue
        meaningful.append((lineno, stripped))
    if not meaningful:
        raise ArtifactError(
            f"{source} is malformed, missing interface line, "
            "the artifact holds no meaningful line"
        )
    first_lineno, first = meaningful[0]
    head = first.split()
    if len(head) != 2 or head[0] != "interface" or not IDENTIFIER.fullmatch(head[1]):
        raise ArtifactError(
            f"{source} is malformed, missing interface line, "
            f"line {first_lineno} does not name an interface"
        )
    interface = head[1]
    terms = {}
    operations = {}
    labels = {}
    excludes = frozenset()
    excludes_line = None
    for lineno, line in meaningful[1:]:
        if line.startswith("label "):
            name, value = _split_statement(line[len("label "):], source, lineno)
            _store(labels, "label", name, value, line, source, lineno)
        elif line.startswith("operation "):
            name, value = _split_statement(line[len("operation "):], source, lineno)
            _store(operations, "operation", name, value, line, source, lineno)
        elif "=" in line and line.split("=", 1)[0].strip() == "excludes":
            if excludes_line is not None:
                raise ArtifactError(
                    f"{source} is malformed, more than one excludes line, "
                    f"second one at line {lineno}"
                )
            _, raw_value = line.split("=", 1)
            excludes = _parse_excludes(raw_value, source, lineno)
            excludes_line = line
        elif "=" in line:
            name, value = _split_statement(line, source, lineno)
            _store(terms, "term", name, value, line, source, lineno)
        else:
            raise ArtifactError(
                f"{source} is malformed, line {lineno} matches no "
                f"statement form, {line}"
            )
    for name in sorted(excludes):
        if name in terms or name in operations:
            raise ArtifactError(
                f"{source} is malformed, it defines {name} and also "
                "excludes it"
            )
    return Artifact(
        source=source,
        interface=interface,
        interface_line=first,
        terms=terms,
        operations=operations,
        labels=labels,
        excludes=excludes,
        excludes_line=excludes_line,
    )


def _split_statement(text, source, lineno):
    if "=" not in text:
        raise ArtifactError(
            f"{source} is malformed, line {lineno} matches no "
            "statement form"
        )
    name, raw_value = text.split("=", 1)
    name = name.strip()
    value = raw_value.strip()
    if not IDENTIFIER.fullmatch(name):
        raise ArtifactError(
            f"{source} is malformed, line {lineno} names no valid "
            f"identifier, {name}"
        )
    if not value:
        raise ArtifactError(
            f"{source} is malformed, line {lineno} states an empty "
            f"value for {name}"
        )
    return name, value


def _store(table, kind, name, value, line, source, lineno):
    if name in table:
        raise ArtifactError(
            f"{source} is malformed, duplicate {kind} name {name} "
            f"at line {lineno}"
        )
    table[name] = Statement(name=name, value=normalize_value(value), line=line)


def _parse_excludes(raw_value, source, lineno):
    names = [part.strip() for part in raw_value.split(",")]
    if not names or any(not IDENTIFIER.fullmatch(name) for name in names):
        raise ArtifactError(
            f"{source} is malformed, line {lineno} excludes line "
            "names no valid identifier"
        )
    return frozenset(names)
