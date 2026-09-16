#!/usr/bin/env bash
set -euo pipefail
umask 077
if [[ $# != 9 ]]; then
  printf 'usage: %s PROBE WORKER HSACO SOURCE PRIVATE_SELECTOR WEIGHTS CASES POLICIES NEW_EVIDENCE\n' "$0" >&2
  exit 2
fi
probe=$1
worker=$2
object=$3
source=$4
selector=$5
weights=$6
cases=$7
policies=$8
evidence=$9
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
baseline="$here/../gfx950-qwen3-kproj-v1"
mkdir -- "$evidence"
mkdir -- "$evidence/runs"
sha256sum "$probe" "$worker" "$object" "$source" "$weights/weights.bf16le" \
  "$here/reference.py" "$here/bind_evidence.py" "$here/verify.sh" \
  "$baseline/reference.py" "$baseline/extract_checkpoint.py" > "$evidence/inputs.sha256"
python3 -B "$here/bind_evidence.py" preflight --weights-dir "$weights" \
  --cases-dir "$cases" --policies-dir "$policies" --output "$evidence/preflight.json" \
  > "$evidence/preflight.log"
"$probe" qwen3-kproj-wave64-inspect --object "$object" --source-file "$source" \
  --metadata "$evidence/artifact.json"
for case_name in zero basis mixed cancellation; do
  "$probe" qwen3-kproj-wave64-run --worker "$worker" --object "$object" --source-file "$source" \
    --metadata "$evidence/artifact.json" --inputs "$cases/$case_name/inputs.bf16le" \
    --weights "$weights/weights.bf16le" --device-id-file "$selector" \
    --run-dir "$evidence/runs/$case_name" --allow-unauthenticated-machine-code
  python3 -B "$here/bind_evidence.py" bind --weights-dir "$weights" \
    --case-dir "$cases/$case_name" --policy-dir "$policies/$case_name" \
    --run-dir "$evidence/runs/$case_name" --artifact "$evidence/artifact.json" \
    --probe "$probe" --worker "$worker" --source "$source" --object "$object" \
    --preflight "$evidence/preflight.json" > "$evidence/$case_name-check.log"
done
sha256sum -c "$evidence/inputs.sha256" > "$evidence/inputs-rechecked.txt"
printf 'Four wave64 GEMV engineering cases completed and checked; no performance claim.\n'
