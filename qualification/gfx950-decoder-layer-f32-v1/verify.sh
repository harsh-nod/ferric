#!/usr/bin/env bash
# Explicit engineering GPU execution, never production qualification.
set -euo pipefail
umask 077

if [[ $# != 6 ]]; then
    printf '%s\n' 'usage: verify.sh PROBE WORKER HSACO SOURCE PRIVATE_DEVICE_ID NEW_EVIDENCE_DIRECTORY' >&2
    exit 2
fi
probe=$1
worker=$2
object=$3
source=$4
selector=$5
evidence=$6
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)

# mkdir intentionally rejects an existing evidence directory. The probe also
# uses exclusive creation for every run, output and artifact manifest.
mkdir -- "$evidence"
mkdir -- "$evidence/runs"
"$probe" inspect --object "$object" --source-file "$source" \
    --metadata "$evidence/artifact.json"

# Freeze every expected result and tolerance before launching any GPU work.
for case in mixed zero saturation skewed-attention; do
    python3 -B -I "$here/reference.py" generate "$evidence/cases/$case" --case "$case"
done

for case in mixed zero saturation skewed-attention; do
    "$probe" run --worker "$worker" --object "$object" --source-file "$source" \
        --metadata "$evidence/artifact.json" \
        --inputs "$evidence/cases/$case/inputs.f32le" \
        --weights "$evidence/cases/$case/weights.f32le" \
        --device-id-file "$selector" --run-dir "$evidence/runs/$case" \
        --allow-unauthenticated-machine-code
    python3 -B -I "$here/reference.py" check "$evidence/cases/$case" \
        "$evidence/runs/$case/output.f32le" \
        --report "$evidence/runs/$case/comparison.json"
done
python3 -B -I "$here/summarize.py" "$evidence" --output "$evidence/summary.json"
