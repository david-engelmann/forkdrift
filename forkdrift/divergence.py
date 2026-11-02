"""Comparison of two parsed artifacts and the divergence report."""

from dataclasses import dataclass

from .artifacts import values_equal
from .errors import InterfaceMismatchError

TERM_VALUE = "term-value"
RESULT_TYPE = "result-type"
EXCLUDED_BUT_DEFINED = "excluded-but-defined"
PRESENCE = "presence"


@dataclass(frozen=True)
class Divergence:
    interface: str
    name: str
    kind: str
    left_source: str
    left_line: str
    right_source: str
    right_line: str
    extra_source: str = None
    extra_line: str = None


def compare_artifacts(left, right):
    """Return every divergence between two artifacts for one interface."""
    if left.interface != right.interface:
        raise InterfaceMismatchError(
            "artifacts name different interfaces, "
            f"{left.source} names {left.interface} and "
            f"{right.source} names {right.interface}"
        )
    found = []
    for kind, left_table, right_table in (
        (TERM_VALUE, left.terms, right.terms),
        (RESULT_TYPE, left.operations, right.operations),
    ):
        for name in sorted(set(left_table) & set(right_table)):
            a = left_table[name]
            b = right_table[name]
            if not values_equal(a.value, b.value):
                found.append(
                    Divergence(left.interface, name, kind,
                               left.source, a.line, right.source, b.line)
                )
    found.extend(_exclusion_divergences(left, right, excluding_is_left=True))
    found.extend(_exclusion_divergences(right, left, excluding_is_left=False))
    found.extend(_presence_divergences(left, right))
    found.sort(key=lambda d: (d.interface, d.name, d.kind))
    return found


def _presence_divergences(left, right):
    """One divergence per name with a statement only one artifact makes.

    A name either artifact excludes is never a presence case, because
    FD-16 gives that name to the exclusion contradiction alone. Label
    lines never take part, because FD-7 declares them non-semantic.
    """
    names = (
        set(left.terms) | set(left.operations)
        | set(right.terms) | set(right.operations)
    )
    out = []
    for name in sorted(names):
        if name in left.excludes or name in right.excludes:
            continue
        left_lines = _one_sided_lines(left, right, name)
        right_lines = _one_sided_lines(right, left, name)
        if not left_lines and not right_lines:
            continue
        out.append(
            _presence_divergence(left, right, name, left_lines, right_lines)
        )
    return out


def _one_sided_lines(defining, other, name):
    """The lines with which defining states a name other does not state.

    The lines come term first and operation second, the order FD-12
    takes defining lines in.
    """
    lines = []
    for own, foreign in (
        (defining.terms, other.terms),
        (defining.operations, other.operations),
    ):
        if name in own and name not in foreign:
            lines.append(own[name].line)
    return lines


def _presence_divergence(left, right, name, left_lines, right_lines):
    """Build the one presence divergence for a name.

    A side with one-sided lines quotes the first in its side row and
    the second as the extra line, and a side with none quotes its
    interface line, so every row quotes a line its source states.
    When both sides hold one-sided lines each holds exactly one, a
    term on one side and an operation on the other, and each side
    quotes its own defining line.
    """
    if left_lines and right_lines:
        return Divergence(left.interface, name, PRESENCE,
                          left.source, left_lines[0],
                          right.source, right_lines[0])
    if left_lines:
        return Divergence(left.interface, name, PRESENCE,
                          left.source, left_lines[0],
                          right.source, right.interface_line,
                          left.source if len(left_lines) > 1 else None,
                          left_lines[1] if len(left_lines) > 1 else None)
    return Divergence(left.interface, name, PRESENCE,
                      left.source, left.interface_line,
                      right.source, right_lines[0],
                      right.source if len(right_lines) > 1 else None,
                      right_lines[1] if len(right_lines) > 1 else None)

def _exclusion_divergences(excluding, defining, excluding_is_left):
    """One divergence per name the excluding side forbids and the other defines.

    The defining side keeps its own side of the report, so a divergence
    reads the same way whichever artifact carries the excludes line.
    """
    out = []
    for name in sorted(excluding.excludes):
        defining_lines = [
            table[name].line
            for table in (defining.terms, defining.operations)
            if name in table
        ]
        if not defining_lines:
            continue
        if excluding_is_left:
            out.append(
                Divergence(excluding.interface, name, EXCLUDED_BUT_DEFINED,
                           excluding.source, excluding.excludes_line,
                           defining.source, defining_lines[0],
                           defining.source if len(defining_lines) > 1 else None,
                           defining_lines[1] if len(defining_lines) > 1 else None)
            )
        else:
            out.append(
                Divergence(excluding.interface, name, EXCLUDED_BUT_DEFINED,
                           defining.source, defining_lines[0],
                           excluding.source, excluding.excludes_line,
                           defining.source if len(defining_lines) > 1 else None,
                           defining_lines[1] if len(defining_lines) > 1 else None)
            )
    return out


def format_report(divergences):
    """Render divergences as the deterministic report text."""
    blocks = []
    for d in divergences:
        block = (
            f"divergence {d.interface} {d.name} {d.kind}\n"
            f"  left ({d.left_source}) {d.left_line}\n"
            f"  right ({d.right_source}) {d.right_line}"
        )
        if d.extra_line is not None:
            extra_side = "left" if d.extra_source == d.left_source else "right"
            block += f"\n  {extra_side} ({d.extra_source}) {d.extra_line}"
        blocks.append(block)
    if not blocks:
        return ""
    return "\n".join(blocks) + "\n"
