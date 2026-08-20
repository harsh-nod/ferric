#!/usr/bin/env python3
"""Enforce same-source Verus coverage for admitted executable modules."""

from __future__ import annotations

import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path


class CoverageError(Exception):
    """The declared proof coverage does not match source or Cargo metadata."""


@dataclass(frozen=True)
class Function:
    owner: str
    name: str
    position: int
    in_verus: bool


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def code_mask(source: str) -> str:
    """Replace comments and literals with spaces while retaining byte offsets."""
    result = list(source)
    cursor = 0
    while cursor < len(source):
        if source.startswith("//", cursor):
            end = source.find("\n", cursor + 2)
            end = len(source) if end < 0 else end
            result[cursor:end] = " " * (end - cursor)
            cursor = end
        elif source.startswith("/*", cursor):
            start = cursor
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
                raise CoverageError("unterminated block comment")
            for index in range(start, cursor):
                if result[index] != "\n":
                    result[index] = " "
        elif source[cursor] in {'"', "'"}:
            quote = source[cursor]
            # A leading apostrophe followed by an identifier is a lifetime.
            if quote == "'" and cursor + 1 < len(source) and (
                source[cursor + 1].isalpha() or source[cursor + 1] == "_"
            ):
                cursor += 1
                continue
            start = cursor
            cursor += 1
            while cursor < len(source):
                if source[cursor] == "\\":
                    cursor += 2
                elif source[cursor] == quote:
                    cursor += 1
                    break
                else:
                    cursor += 1
            else:
                raise CoverageError("unterminated literal")
            for index in range(start, min(cursor, len(result))):
                if result[index] != "\n":
                    result[index] = " "
        else:
            cursor += 1
    return "".join(result)


def closing_brace(code: str, opening: int) -> int:
    depth = 0
    for index in range(opening, len(code)):
        if code[index] == "{":
            depth += 1
        elif code[index] == "}":
            depth -= 1
            if depth == 0:
                return index
    raise CoverageError(f"unclosed brace at byte {opening}")


def spans(code: str, pattern: str) -> list[tuple[int, int, re.Match[str]]]:
    found = []
    for match in re.finditer(pattern, code, re.MULTILINE):
        opening = code.find("{", match.start(), match.end())
        if opening < 0:
            raise CoverageError(f"declaration has no body at byte {match.start()}")
        found.append((opening, closing_brace(code, opening), match))
    return found


def contains(ranges: list[tuple[int, int, object]], position: int) -> bool:
    return any(start < position < end for start, end, _ in ranges)


def parse_functions(path: Path) -> list[Function]:
    source = path.read_text(encoding="utf-8")
    code = code_mask(source)
    verus_ranges = spans(code, r"\bverus\s*!\s*\{")
    test_ranges = spans(
        code,
        r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]\s*mod\s+[A-Za-z_][A-Za-z0-9_]*\s*\{",
    )
    impl_ranges: list[tuple[int, int, str]] = []
    impl_pattern = re.compile(r"\bimpl\b(?P<header>[^{};]*)\{")
    for match in impl_pattern.finditer(code):
        opening = code.find("{", match.start(), match.end())
        header = match.group("header").strip()
        if header.startswith("<"):
            depth = 0
            for index, character in enumerate(header):
                if character == "<":
                    depth += 1
                elif character == ">":
                    depth -= 1
                    if depth == 0:
                        header = header[index + 1 :].strip()
                        break
            else:
                raise CoverageError(f"unclosed impl generic list at byte {match.start()}")
        target = header.rsplit(" for ", 1)[-1].strip()
        owner_match = re.match(r"(?:[A-Za-z_][A-Za-z0-9_]*::)*(?P<owner>[A-Za-z_][A-Za-z0-9_]*)", target)
        if owner_match is None:
            raise CoverageError(f"unsupported impl target at byte {match.start()}: {target}")
        impl_ranges.append((opening, closing_brace(code, opening), owner_match.group("owner")))

    function_pattern = re.compile(
        r"\b(?:(?:pub(?:\([^)]*\))?)\s+)?"
        r"(?:(?:open|closed)\s+)?"
        r"(?:(?P<mode>spec|proof|exec)\s+)?"
        r"(?:const\s+)?(?:unsafe\s+)?fn\s+"
        r"(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*\("
    )
    functions = []
    for match in function_pattern.finditer(code):
        if contains(test_ranges, match.start()) or match.group("mode") in {"spec", "proof"}:
            continue
        owners = [
            (end - start, owner)
            for start, end, owner in impl_ranges
            if start < match.start() < end
        ]
        owner = min(owners)[1] if owners else "-"
        functions.append(
            Function(owner, match.group("name"), match.start(), contains(verus_ranges, match.start()))
        )
    return functions


def parse_manifest(path: Path) -> tuple[dict[str, str], dict[str, dict[tuple[str, str], str]]]:
    modules: dict[str, str] = {}
    declarations: dict[str, dict[tuple[str, str], str]] = {}
    lines = path.read_text(encoding="utf-8").splitlines()
    if not lines or lines[0] != "format=FERRIC-VERIFIED-MODULES-V1":
        raise CoverageError("unsupported verified-module manifest")
    for line_number, line in enumerate(lines[1:], 2):
        fields = line.split("|")
        if line.startswith("module=") and len(fields) == 2:
            package = fields[0].removeprefix("module=")
            source = fields[1]
            if source in modules:
                raise CoverageError(f"duplicate module at line {line_number}")
            modules[source] = package
            declarations[source] = {}
        elif (line.startswith("verified=") or line.startswith("unverified=")) and len(fields) == 3:
            status, source = fields[0].split("=", 1)
            key = (fields[1], fields[2])
            if source not in declarations:
                raise CoverageError(f"function precedes module at line {line_number}")
            if key in declarations[source]:
                raise CoverageError(f"duplicate function at line {line_number}")
            declarations[source][key] = status
        else:
            raise CoverageError(f"malformed line {line_number}")
    if not modules:
        raise CoverageError("manifest contains no modules")
    return modules, declarations


def workspace_modules(repo: Path, metadata: dict[str, object]) -> tuple[dict[str, str], set[str]]:
    workspace_ids = set(metadata["workspace_members"])
    packages = {
        package["name"]: package
        for package in metadata["packages"]
        if package["id"] in workspace_ids
    }
    opted = {
        name
        for name, package in packages.items()
        if package.get("metadata", {}).get("verus", {}).get("verify") is True
    }
    if not opted:
        raise CoverageError("Cargo metadata selects no first-party Verus crates")
    discovered: dict[str, str] = {}
    resolved_repo = repo.resolve()
    for package_name in sorted(opted):
        package = packages[package_name]
        library_targets = [target for target in package["targets"] if "lib" in target["kind"]]
        if len(library_targets) != 1:
            raise CoverageError(f"expected one library target for {package_name}")
        source_root = Path(library_targets[0]["src_path"]).resolve().parent
        try:
            source_root.relative_to(resolved_repo)
        except ValueError as error:
            raise CoverageError(f"library source root escapes repository: {source_root}") from error
        for path in sorted(source_root.rglob("*.rs")):
            resolved = path.resolve()
            try:
                relative = resolved.relative_to(resolved_repo).as_posix()
            except ValueError as error:
                raise CoverageError(f"Rust source escapes repository: {path}") from error
            if relative in discovered:
                raise CoverageError(f"Rust source belongs to multiple packages: {relative}")
            discovered[relative] = package_name
    return discovered, opted


def generate_manifest(repo: Path, metadata: dict[str, object]) -> str:
    modules, _ = workspace_modules(repo, metadata)
    lines = ["format=FERRIC-VERIFIED-MODULES-V1"]
    for source, package in sorted(modules.items()):
        lines.append(f"module={package}|{source}")
        functions = sorted(parse_functions(repo / source), key=lambda item: (item.owner, item.name))
        for function in functions:
            status = "verified" if function.in_verus else "unverified"
            lines.append(f"{status}={source}|{function.owner}|{function.name}")
    return "\n".join(lines) + "\n"


def main() -> None:
    if len(sys.argv) == 5 and sys.argv[1] == "--generate":
        repo = Path(sys.argv[2])
        metadata_path = Path(sys.argv[3])
        output = Path(sys.argv[4])
        try:
            metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
            output.write_text(generate_manifest(repo, metadata), encoding="utf-8")
            print(f"PASS: generated complete coverage manifest at {output}")
        except (OSError, UnicodeError, KeyError, TypeError, ValueError, CoverageError) as error:
            fail(str(error))
        return
    if len(sys.argv) != 4:
        print(
            f"usage: {sys.argv[0]} REPO MANIFEST CARGO_METADATA_JSON\n"
            f"       {sys.argv[0]} --generate REPO CARGO_METADATA_JSON OUTPUT",
            file=sys.stderr,
        )
        raise SystemExit(2)
    repo, manifest_path, metadata_path = map(Path, sys.argv[1:])
    try:
        modules, declarations = parse_manifest(manifest_path)
        metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
        discovered, opted = workspace_modules(repo, metadata)
        if modules != discovered:
            missing = sorted(set(discovered) - set(modules))
            extra = sorted(set(modules) - set(discovered))
            wrong_owner = sorted(
                source
                for source in set(modules) & set(discovered)
                if modules[source] != discovered[source]
            )
            raise CoverageError(
                "verified-module source closure drifted "
                f"(missing={missing}, extra={extra}, wrong_owner={wrong_owner})"
            )
        for source, package in modules.items():
            if package not in opted:
                raise CoverageError(f"{source} belongs to non-verified package {package}")
            path = repo / source
            if not path.is_file():
                raise CoverageError(f"missing verified module {source}")
            actual: dict[tuple[str, str], Function] = {}
            for function in parse_functions(path):
                key = (function.owner, function.name)
                if key in actual:
                    raise CoverageError(f"ambiguous executable function {source}:{key}")
                actual[key] = function
            expected = declarations[source]
            missing = sorted(set(expected) - set(actual))
            extra = sorted(set(actual) - set(expected))
            if missing or extra:
                raise CoverageError(f"{source} coverage drifted (missing={missing}, extra={extra})")
            for key, status in expected.items():
                if (status == "verified") != actual[key].in_verus:
                    raise CoverageError(f"{source}:{key} is not in its declared Verus boundary")
        print(
            "PASS: same-source coverage matched "
            f"({len(modules)} modules, {sum(len(items) for items in declarations.values())} executable bodies, "
            f"opted={','.join(sorted(opted))})"
        )
    except (OSError, UnicodeError, KeyError, TypeError, ValueError, CoverageError) as error:
        fail(str(error))


if __name__ == "__main__":
    main()
