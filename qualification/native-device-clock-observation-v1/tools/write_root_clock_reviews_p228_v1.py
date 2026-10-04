"""Record root's review of actual retained MI350 audits and the explicit capture."""
import hashlib
import json
import os
from pathlib import Path
import sys

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
DIR = L / 'prefix-device-clock-tf4-inputs-v228-v1'


def read(path):
    return json.loads(path.read_bytes())


def save(path, value):
    raw = (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode('ascii')
    with path.open('xb') as stream:
        stream.write(raw)
    return dict(path=str(E / path.relative_to(L)), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def main():
    if sys.flags.optimize or 'PYTHONOPTIMIZE' in os.environ:
        raise RuntimeError('ordinary Python required')
    config = read(L / 'prefix-device-clock-preparation-draft-v228-v1.json')
    review = read(DIR / 'decode-review.draft.json')
    assert review['reviewed'] is False
    for role in ('parent', 'worker'):
        value = read(DIR / (role + '-runtime-review.draft.json'))
        assert value['reviewed'] is False
        assert value['host'] == 'smci350-rck-g03-b19-03'
        assert value['boot_id'] == '2dbbbc5a-d8b7-46ac-bd1f-236e5847e24a'
        value['reviewed'] = True
        value['notes'] = (
            'Root reviewed the actual retained ' + role + ' readelf and ldd streams, their natural '
            'zero exits and independently reaped owners. Dynamic dependencies resolve to the '
            'recorded system libgcc_s, libc and loader, plus libm only for the parent; there '
            'is no RPATH, RUNPATH, unresolved dependency or audit stderr. The exact CPU-selected '
            'ELF bytes are unchanged after transport. Both audits bind the recorded MI350 boot '
            'and ordered GPU identities. This is a compatibility review only: neither selected '
            'executable has been run by these audits, and runtime proofs, arithmetic, numerical '
            'acceptance, clock-domain equivalence and production admission remain unclosed.')
        pin = save(DIR / (role + '-runtime-review.json'), value)
        config[role + '_runtime_review'] = pin
        review[role + '_runtime_review'] = pin
    review['reviewed'] = True
    review['notes'] = (
        'Root reviewed one bounded engineering TF4 capture with the separately qualified '
        'clock V2 parent and worker. The original model, images, numerical inputs and workload '
        'are fixed. Accept only sixteen native clock samples, all 1172 joined dispatch rows, '
        'four byte-identical captures, 152 equal tensor comparisons, all six process/topology '
        'audits and natural Close/reap. Retain failure without an automatic native retry. '
        'This is not the sustained 2048/256 benchmark, independent full-model acceptance, '
        'clock calibration, overlap demonstration or a throughput improvement.')
    review['review_topics'] = dict(
        source_lineage='Actual CPU275 parent and CPU669 worker artifacts have separately retained Cargo '
            'streams and source maps. Parent retains locked Git runtime faaaf15d; worker compiles '
            'sibling27b53. The complete shared worker subtree joins except noncompiled README content.',
        formal='Existing typed lifecycle, state-bank, image and completion checks are preserved. '
            'Engineering machine-code opt-in does not discharge the remaining conditional arithmetic '
            'or runtime proof premises and does not grant production admission.',
        isa='The existing checked-lowering V7 prefix image, all other images and launch geometry '
            'are unchanged. No Down2 replacement, new HSACO, MFMA change or ISA performance claim '
            'is part of this observation. Source-only Down2 work is a separate unadmitted candidate.',
        coherence='Native KFD clock getters run at committed forward boundaries and retain selected '
            'device and queue-epoch identity. All raw queue and allocation checks remain. Raw counters '
            'may be zero or decrease; system clock frequency is not a GPU frequency. No conversion, '
            'cross-device ordering or overlap is inferred. Timestamp fences make this diagnostic '
            'unsuitable as an equal-work host latency optimization comparison.',
        lifecycle='One native attempt keeps pidfd/process-group ownership, fixed resource/deadline '
            'bounds, four complete forwards, consuming Close and natural worker/parent reap. '
            'All three post-audits remain mandatory even after a pre-audit or native failure. '
            'Sample, capture or post-commit validation errors poison the real owner.',
        selected_device='Both actual runtime audits bind smci350-rck-g03-b19-03, boot '
            '2dbbbc5a-d8b7-46ac-bd1f-236e5847e24a and ordered GPU IDs16366993098680759275 and '
            '10838076764495710945. Native admission rechecks the libraries/platform and requires '
            'fresh pre/post topology and process audits. No reset, unrelated process termination '
            'or host substitution is authorized by this review.')
    config['decode_review'] = save(DIR / 'decode-review.json', review)
    print(json.dumps(save(L / 'prefix-device-clock-preparation-final-v228-v1.json', config)))


if __name__ == '__main__':
    main()
