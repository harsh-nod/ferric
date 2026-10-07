"""Create-only retention of the private a004 diagnostic qualification."""
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import stat
import tarfile

D = Path('/tmp/ferric-v16-emitter-b95a642-r1')
O = D / 'gate-up-decode-diagnostic-a004'
ROLES = ('prepare','format','metadata','test','clippy','release')


def require(ok, message):
    if not ok:
        raise ValueError(message)


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    helper = D / 'owner/cpu_profile_42g.py'
    require(digest(helper) == '517671a75602c92beb7f3d43a5584be10a4284d15f68033e616a9462ff7fa224', 'G42 helper')
    q = importlib.util.module_from_spec(importlib.util.spec_from_file_location('retention_g42',helper))
    q.__spec__.loader.exec_module(q)
    q.environment()
    before = q.allocation(32*1024**2)
    files, growth = {}, {}
    def tree(root, prefix):
        require(root.resolve(strict=True) == root, 'canonical retention root')
        for path in root.rglob('*'):
            require(not path.is_symlink() and path.stat().st_uid == 1046, 'owned retention member')
            if not path.is_dir():
                name = prefix + '/' + str(path.relative_to(root))
                require(path.is_file() and name not in files, 'distinct regular member')
                files[name] = path
    tree(O,'qualification')
    for role in ROLES:
        row = json.loads((O / role / 'receipt.json').read_bytes())
        outer_dir = D / ('results/pages-gate-up-decode-' + role + '-a004')
        outer = json.loads((outer_dir / 'result.json').read_bytes())
        require(row['accepted'] and row['returncode'] == 0 and row['original_source_and_elfs_unchanged'], 'inner accepted')
        growth[role] = row['stage_after_bytes']-row['stage_before_bytes']
        require(growth[role] <= row['planning_increment_bytes'], 'phase growth envelope')
        require(outer['status'] == outer['returncode'] == 0 and outer['cleanup_ok'] and outer['child_reaped']
            and not outer['errors'] and not outer['term_sent'] and not outer['kill_sent'], 'clean outer')
        require(outer['profile'] == 'FerricCpuFourCore42GiBEmitterV1'
            and outer['limits']['stage_bytes'] == 42*1024**3, 'G42 profile')
        tree(outer_dir,'outer/' + role)
    release = json.loads((O / 'release/receipt.json').read_bytes())
    require({str(p.relative_to(O / 'source')):digest(p) for p in (O / 'source').rglob('*') if p.is_file()}
        == release['source_after'], 'qualified formatted source closure')
    binary = release['binary']
    require(binary['sha256'] == '634da1c53d46753ffed133be814938a6ebe5075deed72d597fe5ad142de0ed56'
        and digest(Path(binary['path'])) == binary['sha256'], 'new qualified ELF')
    files['binaries/controller-decode'] = Path(binary['path'])
    tree(D / 'inputs/gate-up-decode-diagnostic-a004','inputs')
    for name in ('cpu_profile_42g.py','cpu_guard_42g_emitter.py','cpu-env-42g-emitter.sh'):
        files['owner/' + name] = D / 'owner' / name
    files['original/model-source-a009.json'] = D / 'inputs/gate-up-model-a009/model-source-a009.json'
    os.umask(0o077)
    out = D / 'gate-up-decode-cpu-a004-retained'
    out.mkdir(mode=0o700)
    archive = out / 'evidence.tar.gz'
    members = {}
    with tarfile.open(archive,'x:gz') as packed:
        for name,path in sorted(files.items()):
            info = path.lstat()
            require(path.resolve(strict=True) == path and stat.S_ISREG(info.st_mode)
                and info.st_uid == 1046 and info.st_size < 32*1024**2, 'bounded owned regular member')
            raw = path.read_bytes()
            item = tarfile.TarInfo(name)
            item.size, item.mode, item.mtime = len(raw), stat.S_IMODE(info.st_mode), 0
            members[name] = {'sha256':hashlib.sha256(raw).hexdigest(),'bytes':len(raw),'mode':item.mode}
            packed.addfile(item,io.BytesIO(raw))
    require(archive.stat().st_size < 32*1024**2, 'archive envelope')
    seen = set()
    with tarfile.open(archive) as packed:
        for item in packed:
            require(item.isfile() and item.name in members and item.name not in seen, 'closed archive')
            row = members[item.name]
            require(item.size == row['bytes'] and item.mode == row['mode']
                and hashlib.sha256(packed.extractfile(item).read()).hexdigest() == row['sha256'], 'archive member verified')
            seen.add(item.name)
    require(seen == set(members) and all(digest(files[n]) == r['sha256'] for n,r in members.items()), 'complete stable retention')
    after = q.allocation()
    require(after-before <= 32*1024**2, 'retention growth envelope')
    record = {'schema':'FerricDecodeCpuRetentionR1','accepted':True,'archive_sha256':digest(archive),
        'archive_bytes':archive.stat().st_size,'members':members,'role_growth_bytes':growth,
        'stage_before_bytes':before,'stage_after_bytes':after,'planning_increment_bytes':32*1024**2,
        'native_executed':False,'performance_qualified':False,'cleanup_performed':False}
    q.save(out / 'receipt.json',record)
    print(json.dumps({k:v for k,v in record.items() if k != 'members'},sort_keys=True))


if __name__ == '__main__':
    main()
