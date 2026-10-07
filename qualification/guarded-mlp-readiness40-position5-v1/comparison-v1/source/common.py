"""Bounded, authenticated data reads shared by the module replay and its owner."""
import hashlib
import json
import os
from pathlib import Path
import stat
import types


def require(ok, message):
    if not ok:
        raise ValueError(message)


def parse(raw):
    def pairs(items):
        value = {}
        for key, item in items:
            require(key not in value, 'duplicate JSON key')
            value[key] = item
        return value
    def invalid(_):
        raise ValueError('nonfinite JSON')
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=invalid)


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()


def compact(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def read(path, expected=None, cap=8 << 20):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical input path')
    fields = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink,
                        s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1
                and 0 <= before.st_size <= cap, 'bounded single-link regular input')
        raw = stream.read(cap + 1)
        after = os.fstat(stream.fileno())
    require(fields(before) == fields(after) == fields(path.lstat())
            and len(raw) == before.st_size, 'input identity/content changed')
    row = dict(path=str(path), **compact(raw))
    if expected is not None:
        require(type(expected) is dict and set(expected) in
                ({'bytes', 'sha256'}, {'path', 'bytes', 'sha256'})
                and type(expected['bytes']) is int
                and all(row[key] == value for key, value in expected.items()), 'input pin mismatch')
    return raw, row


def save(path, raw):
    path = Path(path)
    require(type(raw) is bytes and len(raw) <= 8 << 20, 'bounded output')
    with path.open('xb') as stream:
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())
    return dict(path=str(path), **compact(raw))


def load(path, expected, name):
    raw, row = read(path, expected, 1 << 20)
    module = types.ModuleType(name)
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    read(path, row, 1 << 20)
    return module


class Reader:
    def __init__(self):
        self.pins = {}

    def read(self, path, expected=None, cap=8 << 20):
        raw, row = read(path, expected, cap)
        require(row['path'] not in self.pins or self.pins[row['path']] == row, 'readset pin drift')
        self.pins[row['path']] = row
        require(len(self.pins) <= 128, 'bounded readset')
        return raw

    def postcheck(self):
        errors = []
        for row in self.pins.values():
            try:
                read(row['path'], row)
            except BaseException as error:
                errors.append(dict(path=row['path'], error=str(error)))
        return errors
