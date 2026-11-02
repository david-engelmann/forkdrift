"""Command line entry point for forkdrift."""

import argparse
import sys
from pathlib import Path

from .artifacts import parse_artifact
from .divergence import compare_artifacts, format_report, format_sweep
from .errors import ForkdriftError


def _read(path):
    try:
        return Path(path).read_text(encoding="utf-8")
    except UnicodeDecodeError:
        raise ForkdriftError(f"{path} is not readable as utf-8 text")


def main(argv=None):
    parser = argparse.ArgumentParser(
        prog="forkdrift",
        description="Detect semantic divergence between two recorded "
                    "artifacts for one interface.",
    )
    sub = parser.add_subparsers(dest="command", required=True)
    compare_parser = sub.add_parser(
        "compare", help="Compare two recorded artifacts."
    )
    compare_parser.add_argument("left")
    compare_parser.add_argument("right")
    sweep_parser = sub.add_parser(
        "sweep",
        help="Compare one baseline artifact against several others.",
    )
    sweep_parser.add_argument("baseline")
    sweep_parser.add_argument("others", nargs="+")
    args = parser.parse_args(argv)
    if args.command == "compare":
        try:
            left = parse_artifact(_read(args.left), args.left)
            right = parse_artifact(_read(args.right), args.right)
            divergences = compare_artifacts(left, right)
        except ForkdriftError as exc:
            sys.stderr.write(f"error: {exc}\n")
            return 2
        except OSError as exc:
            sys.stderr.write(
                f"error: cannot read {exc.filename}, {exc.strerror}\n"
            )
            return 2
        sys.stdout.write(format_report(divergences))
        return 1 if divergences else 0
    if args.command == "sweep":
        try:
            baseline = parse_artifact(_read(args.baseline), args.baseline)
            others = []
            sweep = []
            for path in args.others:
                other = parse_artifact(_read(path), path)
                sweep.append(compare_artifacts(baseline, other))
                others.append(other)
        except ForkdriftError as exc:
            sys.stderr.write(f"error: {exc}\n")
            return 2
        except OSError as exc:
            sys.stderr.write(
                f"error: cannot read {exc.filename}, {exc.strerror}\n"
            )
            return 2
        sys.stdout.write(format_sweep(baseline, others, sweep))
        return 1 if any(sweep) else 0
    return 2
