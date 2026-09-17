#!/usr/bin/env bash
set -euo pipefail
umask 077

if [[ $# != 8 ]]; then
  printf 'usage: %s PROBE WORKER HSACO SOURCE PRIVATE_DEVICE_ID WEIGHTS_DIR CASES_DIR NEW_EVIDENCE_DIR\n' "$0" >&2
  exit 2
fi
probe=$1
worker=$2
object=$3
source=$4
selector=$5
weights=$6
cases=$7
evidence=$8
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
mkdir -- "$evidence"
mkdir -- "$evidence/runs"
sha256sum "$probe" "$worker" "$object" "$source" "$weights/weights.bf16le" \
  "$script_dir/reference.py" "$script_dir/bind_evidence.py" \
  "$script_dir/extract_checkpoint.py" "$script_dir/verify.sh" > "$evidence/inputs.sha256"
# Recompute all four frozen references and their bounds BEFORE any dispatch.
python3 "$script_dir/bind_evidence.py" preflight --weights-dir "$weights" --cases-dir "$cases" \
  --manifest "$evidence/preflight.json" > "$evidence/preflight.log"
"$probe" qwen3-kproj-inspect --object "$object" --source-file "$source" \
  --metadata "$evidence/artifact.json"
for case_name in zero basis mixed cancellation; do
  "$probe" qwen3-kproj-run --worker "$worker" --object "$object" --source-file "$source" \
    --metadata "$evidence/artifact.json" --inputs "$cases/$case_name/inputs.bf16le" \
    --weights "$weights/weights.bf16le" --device-id-file "$selector" \
    --run-dir "$evidence/runs/$case_name" --allow-unauthenticated-machine-code
  python3 "$script_dir/bind_evidence.py" bind --weights-dir "$weights" \
    --case-dir "$cases/$case_name" --run-dir "$evidence/runs/$case_name" \
    --artifact "$evidence/artifact.json" --probe "$probe" --worker "$worker" \
    --preflight "$evidence/preflight.json" > "$evidence/$case_name-check.log"
done
sha256sum -c "$evidence/inputs.sha256" > "$evidence/inputs-rechecked.txt"
printf 'Four fixed engineering GEMV cases completed and checked; no performance claim.\n'
