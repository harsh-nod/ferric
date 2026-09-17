"""Bounded observation parsing, using the existing M1 validation helpers."""
import hashlib
import importlib.util
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
M1 = ROOT / "adapters/m1-engineering-execution-v1/tools"


def load_module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


LEDGER = load_module("decode_report_m1_ledger", M1 / "performance_ledger.py")
CHECK = LEDGER.CHECK
require = CHECK.require
fields = CHECK.fields
integer = CHECK.integer
hash_value = CHECK.hash_value
statistics = LEDGER.statistics


def digest(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return LEDGER.canonical(value).encode()


def read_json(path, limit=16 * 1024 * 1024):
    raw = CHECK.read_bounded(Path(path), limit)
    return CHECK.json_value(raw), raw


def pinned(pin, base, limit=64 * 1024 * 1024):
    fields(pin, {"path", "sha256"}, "file pin")
    hash_value(pin["sha256"], "file digest")
    require(type(pin["path"]) is str and pin["path"], "file path")
    path = Path(pin["path"])
    if not path.is_absolute():
        path = base / path
    raw = CHECK.read_bounded(path, limit)
    require(digest(raw) == pin["sha256"], "pinned file changed")
    return path, raw


def nearest(values, percent):
    require(values and 0 < percent <= 100, "percentile population")
    return sorted(values)[(len(values) * percent + 99) // 100 - 1]


def percentiles(values):
    return {f"p{p}": nearest(values, p) for p in (50, 90, 99)}


def tool_identities():
    files = list(Path(__file__).parent.glob("*.py"))
    files += [M1 / name for name in ("performance_ledger.py", "compare_tp_batch.py", "host_timing_summary.py")]
    return {str(path.relative_to(ROOT)): digest(path.read_bytes()) for path in files}


def write_new(path, value):
    with path.open("x", encoding="ascii") as stream:
        json.dump(value, stream, sort_keys=True, indent=2, allow_nan=False)
        stream.write("\n")
