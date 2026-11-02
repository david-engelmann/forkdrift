"""Command line entry point for forkdrift."""

import argparse
import sys
from pathlib import Path

from .artifacts import parse_artifact
from .divergence import compare_artifacts, format_report
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
    return 2
