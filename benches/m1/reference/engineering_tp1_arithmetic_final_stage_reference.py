#!/usr/bin/env python3
"""Explicit-mode final-stage reference; never admits or relabels legacy captures."""

import importlib.util
from pathlib import Path
import sys


specification = importlib.util.spec_from_file_location(
    "ferric_tp1_arithmetic_final_stage_core",
    Path(__file__).with_name("engineering_tp1_final_stage_reference.py"),
)
if specification is None or specification.loader is None:
    raise ImportError("cannot load sibling final-stage reference")
reference = importlib.util.module_from_spec(specification)
sys.modules[specification.name] = reference
specification.loader.exec_module(reference)


def run(arguments: list[str]) -> None:
    if len(arguments) != 7:
        raise reference.Failure(
            "usage: engineering_tp1_arithmetic_final_stage_reference.py EXPECTED-ARITHMETIC "
            "CAPTURE WITNESS PINS PINS-SHA256 MODEL-SOURCE NEW-OUTPUT"
        )
    # This identity comes from the caller, never from a captured setup or manifest.
    reference.capture_profile(arguments[0])
    reference.run(arguments[1:], expected_arithmetic=arguments[0])


if __name__ == "__main__":
    try:
        run(sys.argv[1:])
    except (reference.Failure, OSError, ValueError, KeyError, TypeError, AttributeError, OverflowError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        sys.exit(1)
