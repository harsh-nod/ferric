"""Private binary-only decode diagnostic; bounded remote G42 CPU roles."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tarfile
import tomllib

D = Path('/tmp/ferric-v16-emitter-b95a642-r1')
I = D / 'inputs/gate-up-decode-diagnostic-a003'
O = D / 'gate-up-decode-diagnostic-a003'
S = O / 'source'
ORIGINAL = D / 'gate-up-native-library-a003/source'
BIN = 'ferric-qwen3-gate-up-decode-diagnostic'
ALLOW = {'prepare':32,'format':32,'metadata':32,'test':256,'clippy':128,'release':256}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def elf_digest(path):
    # Cargo's release name and deps name are hard links. Only read, never unlink.
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and before.st_uid == 1046 and 0 < before.st_size < 32*1024**2
            and before.st_nlink in (1,2), 'bounded owned Cargo ELF')
    with os.fdopen(os.open(path,os.O_RDONLY|os.O_NOFOLLOW),'rb') as stream:
        opened = os.fstat(stream.fileno())
        digest = hashlib.file_digest(stream,'sha256').hexdigest()
        after = os.fstat(stream.fileno())
    fields = ('st_dev','st_ino','st_mode','st_uid','st_nlink','st_size','st_mtime_ns','st_ctime_ns')
    require(all(getattr(before,k) == getattr(opened,k) == getattr(after,k) == getattr(path.lstat(),k)
                for k in fields), 'stable read-only ELF identity')
    return digest


def main():
    require(len(sys.argv) == 3 and sys.argv[1] in ALLOW, 'fixed role and archive pin')
    role, archive_pin = sys.argv[1:]
    path = D / 'owner/cpu_profile_42g.py'
    require(hashlib.sha256(path.read_bytes()).hexdigest() == '517671a75602c92beb7f3d43a5584be10a4284d15f68033e616a9462ff7fa224', 'qualified profile helper')
    q = importlib.util.module_from_spec(importlib.util.spec_from_file_location('qualified_g42', path))
    q.__spec__.loader.exec_module(q)
    env = q.environment()
    before = q.allocation(ALLOW[role] * 1024**2)
    archive = I / 'source.tar.gz'
    require(q.digest(archive) == archive_pin, 'source archive pin')
    manifest_path = D / 'inputs/gate-up-model-a009/model-source-a009.json'
    require(q.digest(manifest_path) == 'c53c170a8b853e14a7156fd308b97434422152031fed3b8c0d0ca74bc15b1119', 'qualified original manifest')
    original = json.loads(manifest_path.read_bytes())['files']
    def unchanged_original():
        require(all(q.digest(ORIGINAL / n) == row['sha256'] for n, row in original.items()), 'original qualified source unchanged')
        for name, pin in (('live','60bf47551d1ebdfc91f49b529dab8211e3f860a07bdd05921e642b72d505e9a4'),
                          ('counters','fa2049edf8daed70ba268be1b6799f5a4ba42e106e27025f776509876f371231')):
            require(elf_digest(D / ('target-fence-client-a001/release/ferric-qwen3-prefill-width-native-' + name)) == pin,
                    'historical controller ELF unchanged')
    def roster():
        return {str(p.relative_to(S)):q.digest(p) for p in sorted(S.rglob('*')) if not p.is_dir()}
    unchanged_original()
    os.umask(0o077)
    if role == 'prepare':
        O.mkdir(mode=0o700)
    output = O / role
    output.mkdir(mode=0o700)
    record = {'schema':'FerricGateUpDecodeCpuRoleR1','role':role,'accepted':False,
        'archive_sha256':archive_pin,'helper_sha256':q.digest(Path(__file__).resolve()),
        'stage_before_bytes':before,'planning_increment_bytes':ALLOW[role]*1024**2,
        'native_executed':False,'performance_qualified':False}
    try:
        if role == 'prepare':
            S.mkdir(mode=0o700)
            seen, total = set(), 0
            with tarfile.open(archive) as packed:
                for member in packed:
                    name = member.name
                    require(member.isfile() and str(Path(name)) == name and not Path(name).is_absolute()
                            and '..' not in Path(name).parts and name not in seen and member.size < 1024**2,
                            'bounded canonical distinct source member')
                    seen.add(name)
                    total += member.size
                    require(len(seen) <= 128 and total <= 4*1024**2, 'source closure bound')
                    target = S / name
                    target.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
                    with target.open('xb') as stream:
                        stream.write(packed.extractfile(member).read(member.size))
                    target.chmod(stat.S_IMODE(member.mode))
            require(len(seen) == 103 and not (S / 'src/lib.rs').exists() and not (S / 'build.rs').exists(), 'binary-only closure')
            command = ['/bin/true']
        else:
            prior = {'format':'prepare','metadata':'format','test':'metadata','clippy':'test','release':'clippy'}[role]
            receipt = json.loads((O / prior / 'receipt.json').read_bytes())
            require(receipt['accepted'] and receipt['archive_sha256'] == archive_pin
                    and receipt['helper_sha256'] == record['helper_sha256'] and roster() == receipt['source_after'],
                    'exact prior private source and helper')
            outer = json.loads((D / ('results/pages-gate-up-decode-' + prior + '-a003/result.json')).read_bytes())
            require(outer['status'] == 0 and outer['cleanup_ok'] and outer['child_reaped']
                    and not outer['term_sent'] and not outer['kill_sent'], 'clean prior outer')
            common = ['--manifest-path',str(S / 'Cargo.toml')]
            if role == 'format':
                command = [str(q.T / 'bin/cargo'),'fmt',*common]
            elif role == 'metadata':
                command = [str(q.T / 'bin/cargo'),'metadata','--offline','--format-version','1','--features','c1-token-program',*common]
            else:
                common += ['--locked','--offline','--features','c1-token-program','--bin',BIN]
                arguments = {'test':['test',*common,'--','--test-threads=1'],
                    'clippy':['clippy',*common,'--','-D','warnings'],
                    'release':['build','--release',*common]}[role]
                command = [str(q.T / 'bin/cargo'),*arguments]
        record['source_before'] = roster()
        record['argv'] = command
        with (output / 'stdout').open('xb') as stdout, (output / 'stderr').open('xb') as stderr:
            result = subprocess.run(command,cwd=S,env=env,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,timeout=1100,check=False)
        record['returncode'] = result.returncode
        require(result.returncode == 0, 'CPU role failed')
        after = roster()
        changed = {n for n in after if after[n] != record['source_before'].get(n)}
        require(set(after) == set(record['source_before']), 'closed source names')
        require(not changed or (role == 'format' and all(n.endswith('.rs') for n in changed))
                or (role == 'metadata' and changed == {'Cargo.lock'}), 'only declared preparation changes')
        if role == 'metadata':
            old = tomllib.loads((ORIGINAL / 'adapters/m1-engineering-execution-v1/Cargo.lock').read_text())
            new = tomllib.loads((S / 'Cargo.lock').read_text())
            records = lambda rows: {(x['name'],x['version'],x.get('source')):x for x in rows}
            a, b = records(old['package']), records(new['package'])
            # The old root becomes a dependency; its dev-only toml edge moves to the new root.
            adapter = a[('ferric-m1-engineering-execution-v1','0.1.0',None)]
            require(adapter['dependencies'].count('toml') == 1, 'exact original dev-only edge')
            adapter['dependencies'].remove('toml')
            added = set(b)-set(a)
            require(len(added) == 1 and next(iter(added))[0] == 'ferric-gate-up-decode-diagnostic-r1'
                    and all(b.get(k) == v for k,v in a.items()), 'only private package and declared dev-edge relocation in lock')
            record['lock_delta'] = 'private package added; adapter dev-only toml edge moved to private root; all versions, sources, checksums and other edges unchanged'
            metadata = json.loads((output / 'stdout').read_bytes())
            current = [p for p in metadata['packages'] if 'da6b561c5a3f12acc5b0e6da74c808273e728710' in str(p['source'])]
            require(len(current) == 9, 'exact current-wire package count')
            record['current_wire_packages'] = [{'name':p['name'],'source':p['source']} for p in current]
        if role == 'release':
            binary = D / 'target-fence-client-a001/release' / BIN
            record['binary'] = {'path':str(binary),'bytes':binary.stat().st_size,'sha256':elf_digest(binary)}
        record.update(source_after=after,accepted=True)
    except BaseException as error:
        record['error'] = type(error).__name__ + ': ' + str(error)
        raise
    finally:
        try:
            unchanged_original()
            record['original_source_and_elfs_unchanged'] = True
            record['stage_after_bytes'] = q.allocation()
        except BaseException as error:
            record.update(accepted=False,postflight_error=type(error).__name__ + ': ' + str(error))
            raise
        finally:
            q.save(output / 'receipt.json',record)


if __name__ == '__main__':
    main()
