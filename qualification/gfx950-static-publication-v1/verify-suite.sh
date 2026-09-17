#!/usr/bin/env bash
set -euo pipefail
umask 077
if [[ $# != 6 ]]; then
  printf 'usage: %s PROBE WORKER HSACO SOURCE PRIVATE_DEVICE_ID NEW_EVIDENCE_DIR\n' "$0" >&2
  exit 2
fi
probe=$(realpath -- "$1")
worker=$(realpath -- "$2")
object=$(realpath -- "$3")
source=$(realpath -- "$4")
selector=$(realpath -- "$5")
evidence=$(realpath -m -- "$6")
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# This script is expert engineering execution, not an eligibility receipt.
# The operator must separately review the current native memory/profile contract.
python3 -B "$script_dir/reference.py" prepare --directory "$evidence" > /dev/null
"$probe" static-publication-inspect --object "$object" --source-file "$source" --metadata "$evidence/artifact.json"
files=("$probe" "$worker" "$object" "$source" "$script_dir/reference.py" "$0"
       "$evidence/artifact.json" "$evidence/matrix.json")
names=()
for pattern in zero request ready max mixed; do
  for repetition in 0 1 2 3 4 5 6 7; do names+=("$pattern-$repetition"); done
done
names+=(payload_short flags_short payload_empty flags_empty)
for name in "${names[@]}"; do
  files+=("$evidence/$name/case/case.json" "$evidence/$name/case/inputs.f32le"
          "$evidence/$name/case/flags.u32le" "$evidence/$name/case/expected.json")
done
sha256sum -- "${files[@]}" > "$evidence/inputs.sha256"
sha256sum --check --strict "$evidence/inputs.sha256" > "$evidence/preflight.txt"
for name in "${names[@]}"; do
  # Each invocation starts a new disposable worker and five fresh allocations.
  # Failure is terminal: there is no retry after timeout, bad data or cleanup.
  "$probe" static-publication-run --worker "$worker" --object "$object" \
    --source-file "$source" --metadata "$evidence/artifact.json" \
    --case-file "$evidence/$name/case/case.json" \
    --inputs "$evidence/$name/case/inputs.f32le" \
    --initial-flags "$evidence/$name/case/flags.u32le" \
    --device-id-file "$selector" --run-dir "$evidence/$name/run" \
    --allow-unauthenticated-machine-code
  python3 -B "$script_dir/reference.py" check --case-dir "$evidence/$name/case" \
    --run-dir "$evidence/$name/run" --artifact "$evidence/artifact.json" \
    --probe "$probe" --worker "$worker" --object "$object" --source "$source" \
    > "$evidence/$name/check.log"
done
sha256sum --check --strict "$evidence/inputs.sha256" > "$evidence/inputs-rechecked.txt"
# Exit3 plus a retained summary means safe observations but insufficient Ready
# coverage. Do not extend the matrix after seeing that result.
python3 -B "$script_dir/reference.py" summarize --directory "$evidence" \
  --artifact "$evidence/artifact.json" --probe "$probe" --worker "$worker" \
  --object "$object" --source "$source" > "$evidence/summary.log"
