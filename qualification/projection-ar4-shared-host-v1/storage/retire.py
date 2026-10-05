"""Root-run plan/apply for one legacy target's direct Rust cache files only."""
import hashlib
import json
import os
from pathlib import Path
import resource
import shutil
import stat
import sys
import types

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
TARGET = R / 'target-performance-adapter'
DEPS = TARGET / 'debug/deps'
OUT = E / 'performance-adapter-cache-reclaim-v228-v1'
LEGACY = E / 'reclaim_legacy_build_caches_p228_v4.py'
LEGACY_SHA = 'd5f5df32e267d4a614c845353a97892e413d65d5fa6fb24d25d23f3d27c2fc3f'
POLICY = E / 'reclaim_rope_retry_caches_p228_v1.py'
POLICY_SHA = '8378bbcfff5578fc0f4227788ece33714b5c86c23780ecd777cfb6a6ff95d743'
HELPER = E / 'reclaim_projection_decode_caches_p228_v1.py'
HELPER_SHA = 'e80436177cd09e66338950d1aa09773be00447a0c575c539347b6159e84a328a'
SKIP = {'source', 'sources', 'vendor', '.git', '__pycache__', 'tmp', '.venv', 'venv',
        'site-packages', 'private-cache', '.cache', 'crates', 'src', 'rocm-7.2.1-extracted'}
STAT_KEYS = ('st_dev', 'st_ino', 'st_mode', 'st_uid', 'st_gid', 'st_nlink',
             'st_size', 'st_mtime_ns', 'st_ctime_ns', 'st_blocks')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def load(path, digest, name):
    require(path.resolve(strict=True) == path and path.is_file(), 'canonical helper')
    raw = path.read_bytes()
    require(hashlib.sha256(raw).hexdigest() == digest, 'exact reviewed helper')
    module = types.ModuleType(name)
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


def stamp(st):
    return {key: getattr(st, key) for key in STAT_KEYS}


def referenced_paths(value):
    # Protect path references even in old maps or conflicting historical pins.
    if isinstance(value, str):
        path = Path(value)
        if path.is_absolute() and path.is_relative_to(TARGET):
            yield str(path)
    elif isinstance(value, dict):
        for key, item in value.items():
            yield from referenced_paths(key)
            yield from referenced_paths(item)
    elif isinstance(value, list):
        for item in value:
            yield from referenced_paths(item)


def supplemental_name(name):
    return name.endswith('.json') and any(word in name.lower() for word in
        ('manifest', 'candidate', 'retained', 'inputs'))


def receipt_census(a, c, h):
    a.TARGETS = (TARGET,)
    receipts, explicit = a.terminal_receipt_pins(c, h)
    documents = dict(receipts)
    visited, total = 0, sum(pin['bytes'] for pin in documents.values())
    def walk_error(error):
        raise error
    for base, dirs, files in os.walk(R / 'evidence', followlinks=False, onerror=walk_error):
        dirs[:] = sorted(name for name in dirs if name not in SKIP and not name.startswith('target'))
        visited += len(dirs) + len(files)
        require(visited <= 300000, 'bounded supplemental manifest census')
        for name in sorted(name for name in files if supplemental_name(name)):
            path = Path(base) / name
            st = c.ordinary(path)
            require(st.st_size <= 32 << 20, 'bounded nonterminal manifest')
            total += st.st_size
            require(total <= 1 << 30, 'bounded aggregate receipt/manifest bytes')
            documents[str(path)] = h.pin(path)
    for name, record in sorted(documents.items()):
        path = Path(name)
        raw = path.read_bytes()
        require(len(raw) == record['bytes'] and hashlib.sha256(raw).hexdigest() == record['sha256']
                and h.pin(path) == record, 'stable census document')
        explicit.update(referenced_paths(json.loads(raw)))
    return documents, explicit


def eligible(c, path, st, explicit):
    require(stat.S_ISREG(st.st_mode) and not stat.S_ISLNK(st.st_mode), 'ordinary candidate only')
    if (path.parent != DEPS or str(path) in explicit or st.st_nlink != 1
            or st.st_mode & 0o111 or path.suffix not in ('.rlib', '.rmeta', '.o')
            or 'rustc_codegen_fe2o3' in path.name.lower()):
        return False
    return c.disposable(path, st, explicit)


def inventory(c, h, explicit, fixed):
    protected = dict(fixed)
    candidates, directories = [], []
    total, count = 0, 0
    def walk_error(error):
        raise error
    for base, dirs, files in os.walk(TARGET, followlinks=False, onerror=walk_error):
        base = Path(base)
        c.ordinary(base, directory=True)
        directories.append(str(base))
        for name in sorted(dirs):
            c.ordinary(base / name, directory=True)
        count += 1 + len(files)
        require(count <= 200000, 'bounded full target roster')
        for name in sorted(files):
            path = base / name
            st = c.ordinary(path)
            require(stat.S_ISREG(st.st_mode) and st.st_size <= 2 << 30, 'bounded ordinary target body')
            total += st.st_size
            require(total <= 64 << 30, 'bounded full target byte census')
            record = h.pin(path)
            require(stamp(c.ordinary(path)) == stamp(st) and record['bytes'] == st.st_size,
                    'stable target body identity')
            if eligible(c, path, st, explicit):
                candidates.append(dict(**record, stamp=stamp(st), allocated_bytes=st.st_blocks * 512))
            else:
                protected[str(path)] = record
    require(not set(protected).intersection(row['path'] for row in candidates), 'protected disjointness')
    return sorted(candidates, key=lambda row: row['path']), [protected[k] for k in sorted(protected)], sorted(directories)


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ and sys.dont_write_bytecode,
            'ordinary Python -B without optimization')
    require((len(sys.argv) == 2 and sys.argv[1] == 'plan')
            or (len(sys.argv) == 3 and sys.argv[1] == 'apply'), 'plan or apply PLAN_SHA')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'asrock-1w300-g2-2b'
            and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
            'unchanged owned ASROCK/CPU8,9/nice10 boundary')
    for kind, cap in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 180),
                      (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        resource.setrlimit(kind, (cap, cap))
    a = load(LEGACY, LEGACY_SHA, 'legacy_receipt_census')
    c = load(POLICY, POLICY_SHA, 'unchanged_cache_classifier')
    h = load(HELPER, HELPER_SHA, 'unchanged_cache_quiescence')
    require(a.R == R and a.E == c.E == h.E == E and a.POLICY == POLICY
            and a.POLICY_SHA == POLICY_SHA and c.HELPER == HELPER and c.HELPER_SHA == HELPER_SHA,
            'exact helper namespace and dependency chain')
    h.OUT = OUT
    for path in (TARGET, TARGET / 'debug', DEPS):
        c.ordinary(path, directory=True)
    helper_pins = [h.pin(path) for path in (Path(__file__).resolve(), LEGACY, POLICY, HELPER)]
    restricted = h.quiescent((TARGET,))
    documents, explicit = receipt_census(a, c, h)
    fixed = dict(documents)
    fixed.update({row['path']: row for row in helper_pins})
    explicit.update(fixed)
    candidates, protected, directories = inventory(c, h, explicit, fixed)
    h.quiescent((TARGET,))
    common = dict(schema='ferric-performance-adapter-cache-plan-v1', helpers=helper_pins,
        target=str(TARGET), direct_deps=str(DEPS), candidates=candidates, protected=protected,
        directories=directories, documents=documents, explicit_paths=sorted(explicit))
    if sys.argv[1] == 'plan':
        require(candidates, 'nonempty exact disposable selection')
        OUT.mkdir(mode=0o700)
        plan = dict(**common, allocated_bytes=sum(row['allocated_bytes'] for row in candidates),
            free_before=shutil.disk_usage(E).free, restricted_session_processes=restricted,
            terminal_cohort_claimed=False)
        h.save('plan.json', plan)
        print(json.dumps(dict(plan=h.pin(OUT / 'plan.json'), files=len(candidates),
            allocated_bytes=plan['allocated_bytes'], free_before=plan['free_before'])), flush=True)
        return
    c.ordinary(OUT, directory=True)
    require(h.pin(OUT / 'plan.json')['sha256'] == sys.argv[2]
            and not os.path.lexists(OUT / 'complete.json')
            and not os.path.lexists(OUT / 'removed.jsonl'), 'root-selected fresh apply')
    plan = json.loads((OUT / 'plan.json').read_bytes())
    require(all(plan.get(key) == value for key, value in common.items())
            and plan['terminal_cohort_claimed'] is False
            and plan['allocated_bytes'] == sum(row['allocated_bytes'] for row in candidates),
            'exact plan, receipt census, candidates and protected target roster')
    before = shutil.disk_usage(E).free
    h.quiescent((TARGET,))
    # Preserve an unbuffered progress trail if bounded execution stops mid-apply.
    with (OUT / 'removed.jsonl').open('xb', buffering=0) as journal:
        for row in candidates:
            path = Path(row['path'])
            st = c.ordinary(path)
            require(stamp(st) == row['stamp'] and eligible(c, path, st, explicit)
                    and h.pin(path) == {key: row[key] for key in ('path', 'bytes', 'sha256')}
                    and stamp(c.ordinary(path)) == row['stamp'], 'last exact disposable identity')
            path.unlink()
            journal.write((json.dumps(row, sort_keys=True) + '\n').encode('ascii'))
        os.fsync(journal.fileno())
    remaining, after, after_dirs = inventory(c, h, explicit, fixed)
    require(remaining == [] and after == protected and after_dirs == directories,
            'all nonselected target bytes and directories unchanged')
    documents_after, explicit_after = receipt_census(a, c, h)
    require(documents_after == documents and explicit_after | set(fixed) == explicit,
            'all terminal/nonterminal protection documents unchanged')
    for pin in helper_pins:
        require(h.pin(Path(pin['path'])) == pin, 'helper bytes unchanged')
    h.quiescent((TARGET,))
    h.save('complete.json', dict(schema='ferric-performance-adapter-cache-result-v1', passed=True,
        plan=h.pin(OUT / 'plan.json'), removed=h.pin(OUT / 'removed.jsonl'),
        removed_files=len(candidates), removed_allocated_bytes=plan['allocated_bytes'],
        protected_files=len(protected), free_before=before, free_after=shutil.disk_usage(E).free,
        restricted_session_processes=restricted, terminal_cohort_claimed=False,
        source_removed=False, executable_removed=False, shared_library_removed=False,
        code_object_removed=False, non_target_file_removed=False, gpu_execution=False))
    print((OUT / 'complete.json').read_text(), flush=True)


if __name__ == '__main__':
    main()
