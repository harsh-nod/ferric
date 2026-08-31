#!/usr/bin/env python3
"""Publish one authenticated, non-evidence M1 r29 differential intake."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
from typing import Any, NoReturn


VALIDATOR = (
    Path(__file__).resolve().parents[1]
    / "m1"
    / "evidence"
    / "validate-r29-differential-evidence.py"
)


def fail(message: str) -> NoReturn:
    raise SystemExit(f"r29 differential producer: {message}")


def load_validator() -> Any:
    sys.dont_write_bytecode = True
    spec = importlib.util.spec_from_file_location(
        "ferric_m1_r29_differential", VALIDATOR
    )
    if spec is None or spec.loader is None:
        fail("cannot load the source-paired r29 intake validator")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main(arguments: list[str]) -> None:
    if len(arguments) != 2:
        fail("usage: produce-r29-differential-evidence.py INTAKE-ROOT OUTPUT-BUNDLE")
    intake, output = arguments
    load_validator().produce(Path(intake), Path(output))
    print("PASS: published r29 differential partial-non-evidence")


if __name__ == "__main__":
    main(sys.argv[1:])
