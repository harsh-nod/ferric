#!/usr/bin/env python3
"""Fail closed on trust-expanding constructs in bounded Verus sources."""

from __future__ import annotations

import re
import sys
import unicodedata
from pathlib import Path

FORBIDDEN_IDENTIFIERS = {
    "assume_specification",
    "axiom",
    "include",
    "include_bytes",
    "include_str",
    "macro_rules",
    "mod",
    "uninterp",
}

FORBIDDEN_VERIFIER_ATTRIBUTES = {
    "external",
    "external_body",
    "external_fn_specification",
    "external_trait_specification",
    "external_type_specification",
    "trusted",
}


class ScanError(Exception):
    """Proof source is outside Ferric's admitted lexical subset."""


def raw_string_end(source: str, start: int) -> int | None:
    cursor = start
    if source.startswith("br", cursor):
        cursor += 2
    elif source.startswith("r", cursor):
        cursor += 1
    else:
        return None
    hashes = 0
    while cursor < len(source) and source[cursor] == "#":
        hashes += 1
        cursor += 1
    if cursor >= len(source) or source[cursor] != '"':
        return None
    terminator = '"' + "#" * hashes
    end = source.find(terminator, cursor + 1)
    if end < 0:
        raise ScanError("unterminated raw string")
    return end + len(terminator)


def quoted_end(source: str, start: int, quote: str) -> int:
    cursor = start + 1
    while cursor < len(source):
        if source[cursor] == "\\":
            cursor += 2
        elif source[cursor] == quote:
            return cursor + 1
        else:
            cursor += 1
    raise ScanError("unterminated quoted literal")


def code_only(source: str) -> str:
    result: list[str] = []
    cursor = 0
    while cursor < len(source):
        if source.startswith("//", cursor):
            end = source.find("\n", cursor + 2)
            cursor = len(source) if end < 0 else end
        elif source.startswith("/*", cursor):
            depth = 1
            cursor += 2
            while cursor < len(source) and depth:
                if source.startswith("/*", cursor):
                    depth += 1
                    cursor += 2
                elif source.startswith("*/", cursor):
                    depth -= 1
                    cursor += 2
                else:
                    cursor += 1
            if depth:
                raise ScanError("unterminated block comment")
        else:
            raw_end = raw_string_end(source, cursor)
            if raw_end is not None:
                cursor = raw_end
            elif source.startswith('b"', cursor):
                cursor = quoted_end(source, cursor + 1, '"')
            elif source[cursor] == '"':
                cursor = quoted_end(source, cursor, '"')
            elif source.startswith("b'", cursor):
                cursor = quoted_end(source, cursor + 1, "'")
            elif source[cursor] == "'" and cursor + 1 < len(source) and (
                source[cursor + 1].isalpha() or source[cursor + 1] == "_"
            ):
                # Preserve Rust lifetimes and loop labels; they are not literals.
                result.append(source[cursor])
                cursor += 1
            elif source[cursor] == "'":
                cursor = quoted_end(source, cursor, "'")
            else:
                result.append(source[cursor])
                cursor += 1
    return "".join(result)


def lexical_mask(source: str) -> str:
    """Mask non-code bytes while retaining positions for macro extraction."""
    result = list(source)
    cursor = 0
    while cursor < len(source):
        start = cursor
        if source.startswith("//", cursor):
            end = source.find("\n", cursor + 2)
            cursor = len(source) if end < 0 else end
        elif source.startswith("/*", cursor):
            depth = 1
            cursor += 2
            while cursor < len(source) and depth:
                if source.startswith("/*", cursor):
                    depth += 1
                    cursor += 2
                elif source.startswith("*/", cursor):
                    depth -= 1
                    cursor += 2
                else:
                    cursor += 1
            if depth:
                raise ScanError("unterminated block comment")
        else:
            raw_end = raw_string_end(source, cursor)
            if raw_end is not None:
                cursor = raw_end
            elif source.startswith('b"', cursor):
                cursor = quoted_end(source, cursor + 1, '"')
            elif source[cursor] == '"':
                cursor = quoted_end(source, cursor, '"')
            elif source.startswith("b'", cursor):
                cursor = quoted_end(source, cursor + 1, "'")
            elif source[cursor] == "'" and cursor + 1 < len(source) and (
                source[cursor + 1].isalpha() or source[cursor + 1] == "_"
            ):
                cursor += 1
                continue
            elif source[cursor] == "'":
                cursor = quoted_end(source, cursor, "'")
            else:
                cursor += 1
                continue
        for index in range(start, cursor):
            if result[index] != "\n":
                result[index] = " "
    return "".join(result)


def verus_blocks(source: str) -> str:
    mask = lexical_mask(source)
    blocks: list[str] = []
    for match in re.finditer(r"\bverus\s*!\s*\{", mask):
        opening = mask.find("{", match.start(), match.end())
        depth = 0
        for cursor in range(opening, len(mask)):
            if mask[cursor] == "{":
                depth += 1
            elif mask[cursor] == "}":
                depth -= 1
                if depth == 0:
                    blocks.append(source[match.start() : cursor + 1])
                    break
        else:
            raise ScanError("unterminated verus! block")
    if not blocks:
        raise ScanError("source contains no verus! block")
    return "\n".join(blocks)


def scan(path: Path, blocks_only: bool) -> None:
    source = path.read_text(encoding="utf-8")
    normalized = unicodedata.normalize("NFKC", source)
    if normalized != source:
        raise ScanError("source changes under Unicode NFKC normalization")
    for character in source:
        category = unicodedata.category(character)
        forbidden_control = category.startswith("C") and character in {
            "\u200e",
            "\u200f",
            "\u202a",
            "\u202e",
        }
        forbidden_separator = category.startswith("Z") and character not in {
            " ",
            "\n",
        }
        if forbidden_control or forbidden_separator:
            raise ScanError(f"forbidden Unicode category {category}")

    code = code_only(verus_blocks(source) if blocks_only else source)
    identifiers = set(re.findall(r"[A-Za-z_][A-Za-z0-9_]*", code))
    forbidden = sorted(identifiers & FORBIDDEN_IDENTIFIERS)
    if forbidden:
        raise ScanError(f"forbidden proof identifier '{forbidden[0]}'")
    trust_code = re.sub(r"\bfn\s+(?:admit|assume)\s*(?=\()", "fn", code)
    trust_calls = sorted(
        set(re.findall(r"\b(admit|assume)\s*\(", trust_code))
    )
    if trust_calls:
        raise ScanError(f"forbidden trust call '{trust_calls[0]}'")
    verifier_attributes = set(
        re.findall(
            r"#\s*!?\s*\[\s*verifier\s*::\s*([A-Za-z_][A-Za-z0-9_]*)\b",
            code,
        )
    )
    verifier_attributes.update(
        re.findall(
            r"#\s*!?\s*\[\s*(?:verifier|verus_verify)\s*\(\s*"
            r"([A-Za-z_][A-Za-z0-9_]*)\b",
            code,
        )
    )
    forbidden_attributes = sorted(verifier_attributes & FORBIDDEN_VERIFIER_ATTRIBUTES)
    if forbidden_attributes:
        raise ScanError(f"forbidden verifier attribute '{forbidden_attributes[0]}'")
    if re.search(r"#\s*!?\s*\[\s*(?:cfg|cfg_attr)\b", code):
        raise ScanError("conditional proof source is forbidden")


def main() -> int:
    arguments = sys.argv[1:]
    blocks_only = bool(arguments and arguments[0] == "--verus-blocks")
    if blocks_only:
        arguments = arguments[1:]
    if not arguments:
        print(f"usage: {sys.argv[0]} [--verus-blocks] SOURCE...", file=sys.stderr)
        return 2
    for argument in arguments:
        path = Path(argument)
        try:
            scan(path, blocks_only)
        except (OSError, UnicodeError, ScanError) as error:
            print(f"FAIL: {path}: {error}", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
