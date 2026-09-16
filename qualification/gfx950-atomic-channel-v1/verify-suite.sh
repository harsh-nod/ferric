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
evidence=$6
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
mkdir -- "$evidence"
evidence=$(realpath -- "$evidence")
"$probe" atomic-channel-inspect --object "$object" --source-file "$source" \
  --metadata "$evidence/artifact.json"

# Freeze all references and identities before the first device observation.
files=("$probe" "$worker" "$object" "$source" "$script_dir/reference.py"
       "$script_dir/verify-suite.sh" "$evidence/artifact.json")
for case_name in zero walking alternating random; do
  mkdir -- "$evidence/$case_name"
  python3 -B "$script_dir/reference.py" generate --case "$case_name" \
    --case-dir "$evidence/$case_name/case" > "$evidence/$case_name/generate.log"
  files+=("$evidence/$case_name/case/inputs.u32le"
          "$evidence/$case_name/case/expected.json")
done
sha256sum -- "${files[@]}" > "$evidence/inputs.sha256"
sha256sum --check --strict "$evidence/inputs.sha256" > "$evidence/preflight.txt"

# A failure is terminal; no timeout or corrupted result permits a retry.
for case_name in zero walking alternating random; do
  "$probe" atomic-channel-run --worker "$worker" --object "$object" \
    --source-file "$source" --metadata "$evidence/artifact.json" \
    --inputs "$evidence/$case_name/case/inputs.u32le" \
    --device-id-file "$selector" --run-dir "$evidence/$case_name/run" \
    --allow-unauthenticated-machine-code
  python3 -B "$script_dir/reference.py" check --case-dir "$evidence/$case_name/case" \
    --run-dir "$evidence/$case_name/run" --artifact "$evidence/artifact.json" \
    --probe "$probe" --worker "$worker" > "$evidence/$case_name/check.log"
done
sha256sum --check --strict "$evidence/inputs.sha256" > "$evidence/inputs-rechecked.txt"
