#!/usr/bin/env python3
"""Versioned four-forward framework supervisor; root alone authorizes execution."""
import hashlib
import json
import os
from pathlib import Path
import resource
import shutil
import signal
import socket
import stat
import struct
import subprocess
import sys
import time

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
PYTHON = R / 'evidence/qwen3-long-reference-env-v1/venv/bin/python'
SOURCE_AUTH = E / 'authentic-recorder-head-v222-v1/complete.json'
SOURCE_SHA = '28ed08614cf03afa63cf34bdfc3505fde409440a54f6e7acc3f466e25f1188ee'
MANIFEST_SHA = '01c8e0ec31bb215fa35aad27f23c5d09136601b9acf02d0123d487529635595f'
INPUT_TOKENS = [9112,2190,3772,220]
UID = '1b267e8753ea3d92'
INITIAL_FREE = 40 << 30
ONGOING_FREE = 38 << 30
RSS_CAP = 64 << 30
FILE_CAP = OUTPUT_CAP = 32 << 20
CACHE_CAP = 1 << 30
PROVIDER_FILE_CAP = 1 << 30
TMP_CAP = 1 << 30
OTHER_CAP = 256 << 20
OWNED_CAP = CACHE_CAP + TMP_CAP + OTHER_CAP
DEADLINE = 900
CPUS = {8, 9}
ROOT_FIELDS = {'schema','reference_directory','reference_manifest','legacy_plan','source_authentication',
               'token_provenance','execution_review','mode','input_tokens','platform_monitor'}
MONITOR_PID = 284253
MONITOR_PATH = '/usr/local/bin/gpuagent'
MONITOR_BYTES = 28771712
MONITOR_SHA = '5bc2a7d45f2fd992fcf3f53e12b5437a9b3fc7878c653a37ac647c380fb0e913'
MONITOR_PROCESS = {'pid':MONITOR_PID,'uid':0,'ppid':1,'pgid':MONITOR_PID,'session':MONITOR_PID,
                   'start_ticks':1107252,'comm':'gpuagent','cmdline_hex':(MONITOR_PATH+'\0').encode().hex()}
READ_TOOLS = ('/usr/bin/sudo','/usr/bin/timeout','/usr/bin/readlink','/usr/bin/sha256sum')


def require(ok, message):
    if not ok:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def decode(raw):
    def pairs(items):
        value = {}
        for key, item in items:
            require(key not in value, 'duplicate JSON key')
            value[key] = item
        return value
    def invalid(_):
        raise ValueError('nonfinite JSON')
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=invalid)


def canonical(path):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path and not path.is_symlink(), 'canonical path')
    return path


def pin(path, cap=FILE_CAP):
    path = canonical(path)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and 0 <= before.st_size <= cap, 'bounded regular file')
        digest = hashlib.sha256()
        while True:
            block = stream.read(1 << 20)
            if not block:
                break
            digest.update(block)
        after = os.fstat(stream.fileno())
    current = path.stat()
    require(all(getattr(before,k) == getattr(after,k) == getattr(current,k) for k in
                ('st_dev','st_ino','st_size','st_mtime_ns','st_ctime_ns')), 'file changed')
    return {'path':str(path),'bytes':before.st_size,'sha256':digest.hexdigest()}


def checked(row, cap=FILE_CAP):
    require(isinstance(row,dict) and set(row) == {'path','bytes','sha256'} and
            type(row['bytes']) is int and row['bytes'] >= 0, 'file pin shape')
    require(pin(row['path'],cap) == row, 'file pin mismatch')
    raw = Path(row['path']).read_bytes()
    require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'same-buffer file pin')
    return raw


def save(path, value):
    raw = (json.dumps(value,sort_keys=True,indent=2,allow_nan=False)+'\n').encode()
    require(len(raw) <= FILE_CAP, 'JSON file bound')
    with path.open('xb') as stream:
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())
    return pin(path)


def new_directory(path):
    path = Path(path)
    require(path.is_absolute() and path.parent.resolve(strict=True) == path.parent and
            not path.exists() and not path.is_symlink(), 'exclusive output directory')
    path.mkdir(mode=0o700)
    return path


def inventory(path):
    total, cache, temporary, other, output, count, maximum, provider_maximum = 0,0,0,0,0,0,0,0
    if not path.exists():
        return {'total':0,'cache':0,'tmp':0,'other':0,'output':0,'files':0,'maximum_file':0,'maximum_provider_file':0}
    def onerror(error):
        raise error
    for directory, directories, files in os.walk(path,followlinks=False,onerror=onerror):
        for name in [*directories,*files]:
            child = Path(directory)/name
            st = child.lstat()
            relative = child.relative_to(path).parts
            temporary_link = stat.S_ISLNK(st.st_mode) and relative[0] == 'tmp'
            require(stat.S_ISDIR(st.st_mode) or stat.S_ISREG(st.st_mode) or temporary_link,
                    'owned directory link/special file')
            count += 1
            require(count <= 32768, 'owned file roster bound')
            if not stat.S_ISREG(st.st_mode) and not temporary_link:
                continue
            size = st.st_size
            total += size
            provider = ((len(relative) >= 2 and relative[0] == 'tmp') or
                        (len(relative) >= 3 and relative[:2] == ('reference','private-cache')))
            if provider:
                provider_maximum = max(provider_maximum,size)
            else:
                maximum = max(maximum,size)
            if len(relative) >= 3 and relative[:2] == ('reference','private-cache'):
                cache += size
            elif len(relative) >= 2 and relative[0] == 'tmp':
                temporary += size
            else:
                other += size
                if relative[0] == 'reference':
                    output += size
    return {'total':total,'cache':cache,'tmp':temporary,'other':other,'output':output,'files':count,'maximum_file':maximum,
            'maximum_provider_file':provider_maximum}


def storage_ok(value):
    require(value['total'] <= OWNED_CAP, 'owned tree aggregate cap')
    require(value['cache'] <= CACHE_CAP, 'owned provider-cache aggregate cap')
    require(value['tmp'] <= TMP_CAP, 'owned private-tmp aggregate cap')
    require(value['other'] <= OTHER_CAP, 'owned other-files aggregate cap')
    require(value['output'] <= OUTPUT_CAP, 'reference output aggregate cap')
    require(value['maximum_file'] <= FILE_CAP, 'ordinary file/stream cap')
    require(value['maximum_provider_file'] <= PROVIDER_FILE_CAP, 'provider single-file cap')


def parse_proc_stat(raw):
    end = raw.rfind(')')
    require(end > 0, 'proc stat format')
    fields = raw[end+2:].split()
    require(len(fields) >= 22, 'proc stat extent')
    return {'state':fields[0], 'pgid':int(fields[2]), 'session':int(fields[3]),
            'start_ticks':int(fields[19]), 'rss_pages':max(0,int(fields[21]))}


def group_members(pgid):
    rows = []
    for entry in Path('/proc').iterdir():
        if not entry.name.isdecimal():
            continue
        try:
            row = parse_proc_stat((entry/'stat').read_text())
            if row['pgid'] == pgid:
                require(row['session'] == pgid and entry.stat().st_uid == os.getuid(), 'owned group identity')
                rows.append({'pid':int(entry.name), **row})
        except (FileNotFoundError,ProcessLookupError):
            continue
        except PermissionError:
            # Same-uid owned children are readable; unrelated protected processes are not inspected.
            require(entry.stat().st_uid != os.getuid(), 'cannot inspect own process')
    return rows


def child_limits():
    os.sched_setaffinity(0,CPUS)
    os.nice(10)
    resource.setrlimit(resource.RLIMIT_CORE,(0,0))
    # RLIMIT_FSIZE is process-wide; inventory independently keeps ordinary
    # files/streams at32MiB while allowing only private provider files up to1GiB.
    resource.setrlimit(resource.RLIMIT_FSIZE,(PROVIDER_FILE_CAP,PROVIDER_FILE_CAP))


def reap_group(process):
    if process.poll() is None or group_members(process.pid):
        for sig,seconds in ((signal.SIGTERM,2),(signal.SIGKILL,3)):
            try:
                os.killpg(process.pid,sig)
            except ProcessLookupError:
                pass
            end = time.monotonic()+seconds
            while time.monotonic() < end:
                process.poll()
                if process.poll() is not None and not group_members(process.pid):
                    break
                time.sleep(.05)
            if process.poll() is not None and not group_members(process.pid):
                break
    try:
        code = process.wait(timeout=1)
    except subprocess.TimeoutExpired:
        code = None
    return code, not group_members(process.pid)


def bounded(argv, directory, owner, environment, seconds):
    require(os.getuid() != 0 and os.geteuid() != 0, 'inference and ordinary tools must remain unprivileged')
    directory.mkdir(mode=0o700)
    command = save(directory/'command.json',{'argv':argv,'environment':environment,'seconds':seconds,
        'affinity':sorted(CPUS),'nice':10,'group_rss_cap_bytes':RSS_CAP,'file_cap_bytes':FILE_CAP,
        'provider_file_cap_bytes':PROVIDER_FILE_CAP,'rlimit_fsize_bytes':PROVIDER_FILE_CAP,
        'tmp_cap_bytes':TMP_CAP,'other_files_cap_bytes':OTHER_CAP,'owned_tree_cap_bytes':OWNED_CAP,
        'output_cap_bytes':OUTPUT_CAP,'cache_cap_bytes':CACHE_CAP,'initial_free_bytes':INITIAL_FREE,
        'ongoing_free_bytes':ONGOING_FREE,'gpu_execution_authority':'root-reviewed invocation only'})
    started = time.monotonic()
    process, reason, peak, started_pin = None,None,0,None
    with (directory/'stdout').open('xb') as stdout, (directory/'stderr').open('xb') as stderr:
        try:
            process = subprocess.Popen(argv,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,
                env=environment,start_new_session=True,preexec_fn=child_limits)
            started_pin = save(directory/'started.json',{'pid':process.pid,'pgid':process.pid,'supervisor_pid':os.getpid(),
                                                          'command_sha256':command['sha256']})
            while process.poll() is None:
                rows = group_members(process.pid)
                rss = sum(row['rss_pages'] for row in rows)*os.sysconf('SC_PAGE_SIZE')
                peak = max(peak,rss)
                require(rss <= RSS_CAP,'owned process-group RSS cap')
                require(time.monotonic()-started <= seconds,'owned process-group wall deadline')
                require(shutil.disk_usage(owner).free >= ONGOING_FREE,'ongoing disk-free floor')
                storage_ok(inventory(owner))
                time.sleep(.2)
        except BaseException as error:
            reason = type(error).__name__+': '+str(error)
        finally:
            if process is not None:
                if reason is None and process.poll() is not None and group_members(process.pid):
                    reason = 'descendants survived subprocess exit'
                code,absent = reap_group(process)
            else:
                code,absent = None,True
    result = {'exit_code':code,'group_absent':absent,'failure':reason,'elapsed_seconds':time.monotonic()-started,
              'peak_group_rss_bytes':peak,'command':command,'started':started_pin,'stdout':pin(directory/'stdout'),
              'stderr':pin(directory/'stderr'),'performance_measured':False}
    result_pin = save(directory/'result.json',result)
    require(reason is None and code == 0 and absent,'bounded subprocess failed: '+str(result))
    storage_ok(inventory(owner))
    return result,result_pin


def parse_smi_list(value,bdf):
    require(isinstance(value,list) and len(value) == 1 and isinstance(value[0],dict),'one physical GPU framework host')
    row = value[0]
    require(type(row.get('gpu')) is int and row['gpu'] == 0 and row.get('bdf','').lower() == bdf.lower(),
            'SMI index0/physical BDF join')
    return row


def smi_idle(value):
    require(isinstance(value,list) and len(value) == 1 and isinstance(value[0],dict) and
            type(value[0].get('gpu')) is int and value[0]['gpu'] == 0,'SMI process GPU roster')
    rows = value[0].get('process_list')
    require(rows == [] or rows == [{'process_info':'No running processes detected'}], 'GPU has processes/unknown audit schema')


def monitor_document(value, topology):
    require(isinstance(value,dict) and set(value) == {'schema','host','boot_id','process','executable',
            'filesystem_uid','filesystem_gid','read_only_attestation_reviewed'} and
            value['schema'] == 'ferric-p223-platform-monitor-v1','closed monitor identity schema')
    require(value['host'] == topology['host'] and value['boot_id'] == topology['boot_id'], 'monitor host/boot identity')
    process = value['process']
    require(isinstance(process,dict) and set(process) == set(MONITOR_PROCESS), 'monitor process fields')
    require(all(type(process[key]) is type(expected) and process[key] == expected
                for key,expected in MONITOR_PROCESS.items()), 'only exact reviewed gpuagent identity')
    require(value['executable'] == {'path':MONITOR_PATH,'bytes':MONITOR_BYTES,'sha256':MONITOR_SHA}
            and type(value['executable']['bytes']) is int, 'monitor executable pin')
    require(type(value['filesystem_uid']) is int and value['filesystem_uid'] == 1001 and
            type(value['filesystem_gid']) is int and value['filesystem_gid'] == 1001 and
            value['read_only_attestation_reviewed'] is True, 'monitor file owners/read-only review')


def monitor_smi_idle(value):
    require(isinstance(value,list) and len(value) == 1 and isinstance(value[0],dict) and
            set(value[0]) == {'gpu','process_list'} and type(value[0]['gpu']) is int and
            value[0]['gpu'] == 0, 'closed monitor SMI GPU roster')
    rows = value[0]['process_list']
    require(isinstance(rows,list) and len(rows) == 1 and isinstance(rows[0],dict) and
            set(rows[0]) == {'process_info'}, 'exactly one reviewed monitor, no other GPU process')
    info = rows[0]['process_info']
    require(isinstance(info,dict) and set(info) == {'name','pid','memory_usage','mem_usage','usage',
            'cu_occupancy','evicted_time'} and info['name'] == 'N/A' and type(info['pid']) is int and
            info['pid'] == MONITOR_PID, 'closed monitor SMI process identity')
    def zero(item, unit):
        require(isinstance(item,dict) and set(item) == {'value','unit'} and type(item['value']) is int and
                item['value'] == 0 and item['unit'] == unit, 'monitor has nonzero/unknown activity')
    require(isinstance(info['memory_usage'],dict) and set(info['memory_usage']) == {'gtt_mem','cpu_mem','vram_mem'},
            'monitor memory fields')
    for item in info['memory_usage'].values():
        zero(item,'B')
    zero(info['mem_usage'],'B')
    require(isinstance(info['usage'],dict) and set(info['usage']) == {'gfx','enc'}, 'monitor usage fields')
    for item in info['usage'].values():
        zero(item,'ns')
    zero(info['evicted_time'],'ms')
    require(type(info['cu_occupancy']) is int and info['cu_occupancy'] == 0, 'monitor CU occupancy')


def monitor_proc_fields(raw_stat, raw_status, cmdline, proc_uid):
    begin,end = raw_stat.find(' ('),raw_stat.rfind(')')
    require(begin > 0 and end > begin, 'monitor stat identity')
    fields = raw_stat[end+2:].split()
    row = parse_proc_stat(raw_stat)
    require(row['state'] in ('R','S','D','I'), 'monitor no longer a live process')
    uid_rows = [line.split()[1:] for line in raw_status.splitlines() if line.startswith('Uid:')]
    require(len(uid_rows) == 1 and uid_rows[0] == ['0']*4 and proc_uid == 0, 'monitor process UID')
    actual = {'pid':int(raw_stat[:begin]),'uid':proc_uid,'ppid':int(fields[1]),'pgid':row['pgid'],
              'session':row['session'],'start_ticks':row['start_ticks'],'comm':raw_stat[begin+2:end],
              'cmdline_hex':cmdline.hex()}
    require(actual == MONITOR_PROCESS, 'monitor PID/start/owner/cmdline drift')
    return actual


def monitor_process():
    directory = Path('/proc')/str(MONITOR_PID)
    before = monitor_proc_fields((directory/'stat').read_text(),(directory/'status').read_text(),
                                 (directory/'cmdline').read_bytes(),directory.stat().st_uid)
    after = monitor_proc_fields((directory/'stat').read_text(),(directory/'status').read_text(),
                                (directory/'cmdline').read_bytes(),directory.stat().st_uid)
    require(before == after, 'monitor changed during process identity read')
    return before


def read_utility_argv(operation):
    require(operation in ('readlink','sha256sum'), 'only two fixed live-executable read operations')
    return ['/usr/bin/sudo','-n','--','/usr/bin/timeout','--foreground','--kill-after=1','5',
            '/usr/bin/'+operation,'--','/proc/'+str(MONITOR_PID)+'/exe']


def read_utility_members(pgid, retained):
    # sudo may create a separate pty session. Track its observed descendants and
    # their groups without granting this mixed-UID path to ordinary subprocesses.
    snapshot = {}
    for entry in Path('/proc').iterdir():
        if not entry.name.isdecimal():
            continue
        try:
            raw = (entry/'stat').read_text()
            row = parse_proc_stat(raw)
            fields = raw[raw.rfind(')')+2:].split()
            snapshot[int(entry.name)] = {**row,'ppid':int(fields[1]),'uid':entry.stat().st_uid}
        except (FileNotFoundError,ProcessLookupError):
            continue
        except PermissionError:
            require(int(entry.name) not in retained and int(entry.name) != pgid,
                    'cannot verify previously observed read utility')
    selected = {pid for pid,row in snapshot.items() if pid == pgid or row['pgid'] == pgid or
                retained.get(pid,{}).get('start_ticks') == row['start_ticks']}
    while True:
        groups = {snapshot[pid]['pgid'] for pid in selected}
        expanded = selected | {pid for pid,row in snapshot.items() if row['ppid'] in selected or row['pgid'] in groups}
        if selected == expanded:
            break
        selected = expanded
    result = []
    for pid in sorted(selected):
        row = snapshot[pid]
        require(pid != MONITOR_PID and row['pgid'] != MONITOR_PID and row['uid'] in (0,os.getuid()),
                'read utility process ownership')
        if pid in retained:
            require(retained[pid]['start_ticks'] == row['start_ticks'], 'read utility PID reuse')
        retained[pid] = row
        result.append({'pid':pid,**row})
    return result


def live_executable_read(operation, directory, owner, env):
    require(os.getuid() != 0 and os.geteuid() != 0, 'controller must not run as root')
    argv = read_utility_argv(operation)
    directory.mkdir(mode=0o700)
    command = save(directory/'command.json',{'argv':argv,'seconds':10,'inner_timeout_seconds':5,
        'inner_kill_after_seconds':1,'read_only':True,'gpu_execution':False,'platform_monitor_signalled':False})
    started,process,reason,retained = time.monotonic(),None,None,{}
    code,absent = None,False
    with (directory/'stdout').open('xb') as stdout,(directory/'stderr').open('xb') as stderr:
        try:
            process = subprocess.Popen(argv,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,env=env,
                                       start_new_session=True,preexec_fn=child_limits)
            save(directory/'started.json',{'pid':process.pid,'pgid':process.pid,'supervisor_pid':os.getpid(),
                                           'command_sha256':command['sha256']})
            while True:
                members = read_utility_members(process.pid,retained)
                require(sum(x['rss_pages'] for x in members)*os.sysconf('SC_PAGE_SIZE') <= 256 << 20,
                        'read utility RSS bound')
                require(time.monotonic()-started <= 10, 'read utility outer deadline')
                require(shutil.disk_usage(owner).free >= ONGOING_FREE, 'read utility disk floor')
                require((directory/'stdout').stat().st_size <= 4096 and (directory/'stderr').stat().st_size <= 4096,
                        'read utility output bound')
                if process.poll() is not None and not members:
                    break
                time.sleep(.05)
            code,absent = process.wait(timeout=1),True
        except BaseException as error:
            reason = type(error).__name__+': '+str(error)
        finally:
            if process is not None and not absent:
                # The root timeout bounds its fixed child. Never sudo kill, signal
                # gpuagent, or use the inference cleanup path for a privileged tree.
                try:
                    process.wait(timeout=7)
                except subprocess.TimeoutExpired:
                    try:
                        process.terminate()
                    except (ProcessLookupError,PermissionError):
                        pass
                try:
                    code = process.wait(timeout=1)
                except subprocess.TimeoutExpired:
                    code = None
                try:
                    absent = not read_utility_members(process.pid,retained)
                except BaseException as error:
                    absent = False
                    reason = (reason or '')+'; utility absence check: '+type(error).__name__+': '+str(error)
    result = {'exit_code':code,'observed_utility_tree_absent':absent,'failure':reason,
        'elapsed_seconds':time.monotonic()-started,'command':command,'observed_processes':retained,
        'stdout':pin(directory/'stdout'),'stderr':pin(directory/'stderr'),'read_only':True,
        'platform_monitor_signalled':False,'gpu_execution':False}
    result_pin = save(directory/'result.json',result)
    require(reason is None and code == 0 and absent, 'live-executable read failed; no inference permitted')
    return checked(result['stdout'],4096),result_pin


def validate_live_executable(readlink_output, sha_output):
    require(readlink_output == (MONITOR_PATH+'\n').encode(), 'live executable path changed')
    require(sha_output == (MONITOR_SHA+'  /proc/'+str(MONITOR_PID)+'/exe\n').encode(),
            'live executable bytes changed')


def monitor_audit(label, value, topology, owner, env):
    monitor_document(value,topology)
    before = monitor_process()
    executable = checked(value['executable'])
    st = Path(MONITOR_PATH).stat()
    require(st.st_uid == value['filesystem_uid'] and st.st_gid == value['filesystem_gid'], 'monitor file owner changed')
    require(len(executable) == MONITOR_BYTES, 'monitor executable size')
    link,link_pin = live_executable_read('readlink',owner/(label+'-live-path'),owner,env)
    digest,digest_pin = live_executable_read('sha256sum',owner/(label+'-live-sha'),owner,env)
    validate_live_executable(link,digest)
    result,result_pin = bounded(['/usr/bin/amd-smi','process','--json'],owner/(label+'-process'),owner,env,30)
    monitor_smi_idle(decode(checked(result['stdout'])))
    require(monitor_process() == before and device() == topology, 'monitor/topology changed during audit')
    return {'process':before,'live_path_result':link_pin,'live_sha_result':digest_pin,'smi_result':result_pin,
            'sample':idle_sample(),'exclusive_reservation':False}


def device():
    card = Path('/sys/class/drm/card1/device')
    require((card/'unique_id').read_text().strip() == UID,'fresh card1 UID')
    selected = []
    for node in Path('/sys/class/kfd/kfd/topology/nodes').iterdir():
        if not node.name.isdecimal():
            continue
        properties = dict(line.split() for line in (node/'properties').read_text().splitlines())
        if int(properties.get('unique_id','0')) == int(UID,16):
            require(int(properties['gfx_target_version']) == 90500 and
                    int(properties['wave_front_size']) == 64 and int(properties['vendor_id']) == 4098,
                    'fresh gfx950 Wave64 KFD mapping')
            render = Path('/sys/class/drm')/('renderD'+properties['drm_render_minor'])/'device'
            require(render.resolve(strict=True) == card.resolve(strict=True),'KFD/DRM physical join')
            selected.append({'node':str(node),'gpu_id':int((node/'gpu_id').read_text()),'properties':properties})
    require(len(selected) == 1,'one current KFD UID')
    return {'host':socket.gethostname(),'boot_id':Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
            'uid':UID,'bdf':card.resolve(strict=True).name,'card':'card1','kfd':selected[0]}


def idle_sample():
    card = Path('/sys/class/drm/card1/device')
    row = {name:int((card/name).read_text().strip()) for name in
           ('gpu_busy_percent','mem_info_vram_used','mem_info_vram_total')}
    require(row['gpu_busy_percent'] == 0 and row['mem_info_vram_total']-row['mem_info_vram_used'] >= 48 << 30,
            'idle sample/free GPU memory')
    row['monotonic_ns'] = time.monotonic_ns()
    return row


def reviewed(value,inputs,topology):
    fields = {'schema','reference_manifest_sha256','mode','input_tokens','host','boot_id','gpu_unique_id','bdf',
              'source_and_policy_reviewed','device_mapping_reviewed','shared_host_idle_window_reviewed',
              'dedicated_disposable_process_reviewed','no_candidate_inputs','exclusive_reservation',
              'numerical_acceptance','performance_claim','production_authority','platform_monitor_sha256',
              'pinned_zero_allocation_monitor_reviewed','narrow_read_only_sudo_reviewed'}
    require(isinstance(value,dict) and set(value) == fields and
            value['schema'] == 'ferric-p224-rearm-four-framework-execution-review-v1','explicit root review schema')
    require(value['reference_manifest_sha256'] == MANIFEST_SHA and value['mode'] == inputs['mode'] and
            value['input_tokens'] == inputs['input_tokens'] and value['host'] == topology['host'] and
            value['boot_id'] == topology['boot_id'] and value['gpu_unique_id'] == UID and
            value['bdf'] == topology['bdf'] and value['platform_monitor_sha256'] == inputs['platform_monitor']['sha256'],
            'review/input/current-device/monitor join')
    for field in ('source_and_policy_reviewed','device_mapping_reviewed','shared_host_idle_window_reviewed',
                  'dedicated_disposable_process_reviewed','no_candidate_inputs','pinned_zero_allocation_monitor_reviewed',
                  'narrow_read_only_sudo_reviewed'):
        require(value[field] is True,'explicit reviewed precondition '+field)
    for field in ('exclusive_reservation','numerical_acceptance','performance_claim','production_authority'):
        require(value[field] is False,'review cannot claim '+field)


def reference_package(inputs):
    directory = canonical(inputs['reference_directory'])
    require(inputs['reference_manifest']['path'] == str(directory/'manifest.json') and
            inputs['reference_manifest']['sha256'] == MANIFEST_SHA,'frozen reference manifest')
    manifest = decode(checked(inputs['reference_manifest']))
    expected = {'framework_reference.py','diagnostics.py','policy.json','README.txt','test_framework_reference.py',
                'helpers/long_reference.py'}
    require({row['path'] for row in manifest['files']} == expected and len(manifest['files']) == 6,
            'exact reference source roster')
    pins = [inputs['reference_manifest']]
    for row in manifest['files']:
        actual = {'path':str(directory/row['path']),'bytes':row['bytes'],'sha256':row['sha256']}
        checked(actual)
        pins.append(actual)
    return directory,pins


def environment():
    return {'PATH':'/usr/bin:/bin','LANG':'C.UTF-8','HOME':str(R/'evidence/qwen3-long-reference-env-v1/home'),
            'TMPDIR':str(R/'evidence/qwen3-long-reference-env-v1/tmp'),
            'HIP_VISIBLE_DEVICES':'0','CUDA_VISIBLE_DEVICES':'0','HF_HUB_OFFLINE':'1',
            'TRANSFORMERS_OFFLINE':'1','HF_DATASETS_OFFLINE':'1','TOKENIZERS_PARALLELISM':'false',
            'OMP_NUM_THREADS':'2','OPENBLAS_NUM_THREADS':'2','MKL_NUM_THREADS':'2','PYTHONDONTWRITEBYTECODE':'1'}


def validate_selection(mode,tokens):
    require(mode == 'teacher_forced' and isinstance(tokens,list) and tokens == INPUT_TOKENS
            and all(type(token) is int for token in tokens),'closed four-token teacher-forced profile')


def validate_reference_roster(report):
    require(isinstance(report.get('passes'),list) and len(report['passes']) == 2,'two fresh repeat passes')
    for ordinal,repeat in enumerate(report['passes'],1):
        require(repeat['ordinal'] == ordinal and repeat['fresh_cache'] is True
                and len(repeat['cases']) == 4,'four forwards per fresh repeat pass')
        for position,case in enumerate(repeat['cases']):
            record = case['record']
            require(record['generation'] == position+1 and record['position'] == position
                    and record['input_token'] == INPUT_TOKENS[position]
                    and type(record['output_token']) is int and 0 <= record['output_token'] < 151936
                    and len(case['cache_sha256']) == 36,'four-forward position/input/cache join')


def main(mode,input_path,input_sha,output):
    require(mode in ('--inspect','--run-reviewed'),'explicit controller mode')
    require(os.getuid() != 0 and os.geteuid() != 0,'controller and inference must not run as root')
    own = pin(Path(__file__).resolve(strict=True))
    input_pin = pin(input_path)
    require(input_pin['sha256'] == input_sha,'launch inputs SHA')
    inputs = decode(checked(input_pin))
    require(isinstance(inputs,dict) and set(inputs) == ROOT_FIELDS and
            inputs['schema'] == 'ferric-p224-rearm-four-framework-launch-inputs-v1','launch input schema')
    validate_selection(inputs['mode'],inputs['input_tokens'])
    source,pins = reference_package(inputs)
    require(inputs['source_authentication']['path'] == str(SOURCE_AUTH) and
            inputs['source_authentication']['sha256'] == SOURCE_SHA,'authentic source receipt')
    for row in [inputs['legacy_plan'],inputs['source_authentication'],inputs['execution_review'],
                inputs['token_provenance']['manifest'],inputs['token_provenance']['prompt_ids'],inputs['platform_monitor']]:
        checked(row)
        pins.append(row)
    pins.extend([own,input_pin])
    require(CPUS <= os.sched_getaffinity(0),'CPUs8/9 are available')
    require(shutil.disk_usage(Path(output).parent).free >= INITIAL_FREE,'initial disk-free floor')
    owner = new_directory(output)
    save(owner/'inputs.json',inputs)
    env = environment()
    for name in ('home','tmp'):
        (owner/name).mkdir(mode=0o700)
    env['HOME'],env['TMPDIR'] = str(owner/'home'),str(owner/'tmp')
    # Reuse the existing venv through its original symlink, preserving sys.prefix.
    python_identity = {'path':str(PYTHON),'link':os.readlink(PYTHON) if PYTHON.is_symlink() else None,
                       'target':pin(PYTHON.resolve(strict=True),128 << 20)}
    save(owner/'python-identity.json',python_identity)
    tests,test_pin = bounded(['/usr/bin/python3','-B','-m','unittest','discover','-v','-s',str(source),
                             '-p','test_framework_reference.py'],owner/'cpu-tests',owner,env,120)
    test_text = checked(tests['stderr']).decode('utf-8')
    require('Ran 13 tests in ' in test_text and '\nOK\n' in test_text and 'skipped=' not in test_text,'all13 CPU tests')
    topology = device()
    review = decode(checked(inputs['execution_review']))
    reviewed(review,inputs,topology)
    monitor = decode(checked(inputs['platform_monitor']))
    monitor_document(monitor,topology)
    pins.append(monitor['executable'])
    read_tools = [pin(Path(tool),128 << 20) for tool in READ_TOOLS]
    pins.extend(read_tools)
    save(owner/'topology-before.json',topology)
    smi_pin = pin(Path('/usr/bin/amd-smi').resolve(strict=True),128 << 20)
    result,_ = bounded(['/usr/bin/amd-smi','list','--json'],owner/'pre-list',owner,env,30)
    parse_smi_list(decode(checked(result['stdout'])),topology['bdf'])
    samples = []
    for sample in range(3):
        require(device() == topology,'topology changed during idle review')
        samples.append(monitor_audit('idle-'+str(sample),monitor,topology,owner,env))
        if sample != 2:
            time.sleep(1)
    save(owner/'idle-before.json',{'samples':samples,'exclusive_reservation':False})
    plan = {'schema':'ferric-p224-rearm-four-framework-reference-plan-v1',
        'model_id':'f18fc461576d1a3053a19aba5946ef7b3b45aaf7cbb45d77f5c276f18567224a',
        'bundle_id':'6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b',
        'harness_sha256':pin(source/'framework_reference.py')['sha256'],'diagnostics':pin(source/'diagnostics.py'),
        'policy':pin(source/'policy.json'),'legacy_plan':inputs['legacy_plan'],'mode':inputs['mode'],
        'input_tokens':inputs['input_tokens'],'token_provenance':inputs['token_provenance'],
        'source_authentication':inputs['source_authentication'],'host':topology['host'],'boot_id':topology['boot_id'],
        'gpu_unique_id':UID,'gpu_unique_id_file':'/sys/class/drm/card1/device/unique_id','hip_visible_devices':'0',
        'python_executable':str(PYTHON),'python_prefix':str(PYTHON.parent.parent),
        'model_root':str(R/'models/qwen3-source-v1/target'),'output_root':str(owner/'reference'),
        'supervisor_pid':os.getpid(),'execution_review':inputs['execution_review'],
        'resources':{'timeout_seconds':900,'host_rss_cap_bytes':RSS_CAP,'minimum_free_host_bytes':48 << 30,
                     'minimum_free_gpu_bytes':48 << 30,'output_cap_bytes':OUTPUT_CAP,'cache_cap_bytes':CACHE_CAP}}
    plan_pin = save(owner/'plan.json',plan)
    argv = [str(PYTHON),'-I','-B',str(source/'framework_reference.py')]
    inspection,inspection_pin = bounded([*argv,'--inspect',plan_pin['path'],plan_pin['sha256']],
                                       owner/'inspect',owner,env,60)
    inspection_value = decode(checked(inspection['stdout']))
    require(inspection_value['schema'] == 'ferric-p224-rearm-four-framework-reference-inspection-v1' and
            inspection_value['gpu_opened'] is False and inspection_value['input_tokens'] == inputs['input_tokens'],
            'no-GPU reference inspection')
    for row in pins:
        checked(row)
    require(device() == topology,'topology changed before execution')
    output_pin = None
    if mode == '--run-reviewed':
        # Repeat process audit immediately before the one permitted GPU reference.
        save(owner/'immediate-idle.json',monitor_audit('immediate',monitor,topology,owner,env))
        execution,execution_pin = bounded([*argv,'--run-reviewed-framework-reference',plan_pin['path'],plan_pin['sha256']],
                                         owner/'execute',owner,env,DEADLINE)
        output_pin = pin(owner/'reference/reference.json')
        require(decode(checked(execution['stdout'])) == output_pin,'execution stdout/reference pin join')
        report = decode(checked(output_pin))
        require(report['schema'] == 'ferric-p224-rearm-four-framework-reference-v1' and report['status'] == 'PASS' and
                report['repeat_passes_byte_equal'] is True and report['numerical_acceptance'] is False and
                report['mode'] == inputs['mode'] and report['selected_input_tokens'] == inputs['input_tokens'],
                'qualified repeated reference, not candidate acceptance')
        validate_reference_roster(report)
        require(plan_pin in report['input_pins'] and inputs['execution_review'] in report['input_pins'] and
                inputs['source_authentication'] in report['input_pins'],'reference plan/review/source custody')
        for repeat in report['passes']:
            for case in repeat['cases']:
                require(case['payload']['bytes'] == 606976 and Path(case['payload']['path']).parent == owner/'reference',
                        'bounded captured forward payload')
                checked(case['payload'])
        save(owner/'idle-after.json',monitor_audit('post',monitor,topology,owner,env))
    else:
        execution_pin = None
    require(device() == topology,'final topology changed')
    for row in pins:
        checked(row)
    require(pin(PYTHON.resolve(strict=True),128 << 20) == python_identity['target'] and
            (os.readlink(PYTHON) if PYTHON.is_symlink() else None) == python_identity['link'],'Python changed')
    require(pin(Path('/usr/bin/amd-smi').resolve(strict=True),128 << 20) == smi_pin,'SMI tool changed')
    storage_ok(inventory(owner))
    return save(owner/'complete.json',{'schema':'ferric-p224-rearm-four-framework-launch-complete-v1','passed':True,
        'mode':mode,'input_pins':pins,'plan':plan_pin,'cpu_tests':13,'cpu_result':test_pin,'inspection_result':inspection_pin,
        'execution_result':execution_pin,'reference':output_pin,'gpu_execution':mode == '--run-reviewed',
        'owned_group_reaped':True,'reference_process_exit_observed':mode == '--run-reviewed',
        'numerical_acceptance':False,'candidate_gpu_execution':False,'production_authority':False,
        'performance_measured':False,'exclusive_reservation':False,'topology':topology,
        'platform_monitor':inputs['platform_monitor'],'monitor_exception':'exact-pinned-zero-allocation-only',
        'inference_unprivileged':True,'platform_monitor_signalled':False,'observed_read_utility_trees_absent':True})


if __name__ == '__main__':
    require(len(sys.argv) == 5,'launch.py (--inspect|--run-reviewed) INPUTS SHA NEW_OUTPUT_DIR')
    print(json.dumps(main(*sys.argv[1:]),sort_keys=True))
