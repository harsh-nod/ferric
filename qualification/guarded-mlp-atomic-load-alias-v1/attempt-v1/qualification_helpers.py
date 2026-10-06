"""Source-only S/RPO qualification helpers; no command execution or admission."""

from pathlib import Path
import re


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def full_suite_outcomes(path, expected_names, expected_ignored):
    """Replay a full default suite against separately authenticated inventories."""
    for names in (expected_names, expected_ignored):
        require(isinstance(names, (list, tuple))
                and all(isinstance(name, str) and re.fullmatch(r'[A-Za-z0-9_:]+', name)
                        for name in names)
                and len(names) == len(set(names)), 'invalid expected libtest roster')
    require(expected_names and set(expected_ignored) <= set(expected_names),
            'ignored roster must be a subset of the full inventory')
    text = Path(path).read_text()
    pattern = re.compile(r'test ([A-Za-z0-9_:]+)(?: - should panic)? '
                         r'\.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?')
    progress_pattern = re.compile(
        r'test ([A-Za-z0-9_:]+) has been running for over 60 seconds')
    expected, ignored = set(expected_names), set(expected_ignored)
    named, completed, progress_names = [], set(), set()
    summary_seen = False
    for line in text.splitlines():
        if line.startswith('test result:'):
            summary_seen = True
            continue
        if not line.startswith('test '):
            continue
        progress = progress_pattern.fullmatch(line)
        if progress is not None:
            name = progress[1]
            require(name in expected and name not in ignored
                    and name not in completed and name not in progress_names
                    and not summary_seen, 'invalid named libtest progress notice')
            progress_names.add(name)
            continue
        match = pattern.fullmatch(line)
        require(match is not None, 'malformed named libtest result')
        named.append((match[1], match[2]))
        completed.add(match[1])
    require(len(named) == len(expected_names)
            and sorted(name for name, _ in named) == sorted(expected_names),
            'full inventory must appear exactly once')
    ignored = set(expected_ignored)
    require(all(status == ('ignored' if name in ignored else 'ok') for name, status in named),
            'full-suite status differs from the authenticated ignored roster')
    summaries = re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; '
                           r'(\d+) ignored; (\d+) measured; (\d+) filtered out;', text, re.M)
    require(sum(line.startswith('test result:') for line in text.splitlines()) == 1
            and summaries == [('ok', str(len(expected_names) - len(ignored)), '0',
                               str(len(ignored)), '0', '0')],
            'full-suite summary must match every named outcome without filtering')
    return dict(names=sorted(expected_names), passed=len(expected_names) - len(ignored),
                failed=0, ignored=len(ignored), filtered_out=0,
                ignored_names=sorted(ignored), named_outcomes=dict(sorted(named)))


def select_backend_rlib(records, source, target_root, pin):
    """Select only the final non-test backend rlib from checked Cargo JSON rows."""
    source, target_root = Path(source), Path(target_root)
    package = source / 'crates' / 'rustc-codegen-fe2o3'
    rows = [row for row in records if row.get('reason') == 'compiler-artifact'
            and row.get('manifest_path') == str(package / 'Cargo.toml')
            and row.get('target', {}).get('name') == 'rustc_codegen_fe2o3'
            and sorted(row['target'].get('kind', [])) == ['dylib', 'rlib']
            and sorted(row['target'].get('crate_types', [])) == ['dylib', 'rlib']
            and row['target'].get('src_path') == str(package / 'src' / 'lib.rs')
            and row.get('profile', {}).get('test') is False]
    require(len(rows) == 1, 'unique non-test backend library artifact required')
    row = rows[0]
    require(row.get('executable') is None, 'backend library must not be executable')
    paths = [Path(path) for path in row['filenames'] if path.endswith('.rlib')]
    require(len(paths) == 1, 'one selected backend rlib required')
    selected = paths[0]
    require(selected.name == 'librustc_codegen_fe2o3.rlib'
            and selected.is_absolute() and selected.is_relative_to(target_root)
            and selected.resolve(strict=True) == selected and selected.is_file(),
            'selected backend rlib outside fresh target or wrong filename')
    with selected.open('rb') as stream:
        require(stream.read(8) == b'!<arch>\n', 'selected backend rlib is not an ordinary archive')
    return dict(pin=pin(selected), cargo_artifact=row)
