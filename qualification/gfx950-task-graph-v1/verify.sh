#!/usr/bin/env bash
set -euo pipefail
umask 077

if [[ $# != 7 ]]; then
  printf 'usage: %s PROBE WORKER HSACO SOURCE PRIVATE_DEVICE_ID NEW_EVIDENCE_DIR CASE\n' "$0" >&2
  exit 2
fi
probe=$1
worker=$2
object=$3
source=$4
selector=$5
evidence=$6
case_name=$7
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
case "$case_name" in
  zero|ramp|maximum|random) ;;
  *) printf 'unknown fixed task-graph case\n' >&2; exit 2 ;;
esac
mkdir -- "$evidence"
"$probe" task-graph-inspect --object "$object" --source-file "$source" \
  --metadata "$evidence/artifact.json"
python3 "$script_dir/reference.py" generate --case "$case_name" --case-dir "$evidence/case" \
  > "$evidence/generate.log"
"$probe" task-graph-run --worker "$worker" --object "$object" --source-file "$source" \
  --metadata "$evidence/artifact.json" --inputs "$evidence/case/inputs.u32le" \
  --device-id-file "$selector" --run-dir "$evidence/run" --allow-unauthenticated-machine-code
python3 "$script_dir/reference.py" check --case-dir "$evidence/case" --run-dir "$evidence/run" \
  --artifact "$evidence/artifact.json" --probe "$probe" --worker "$worker"
