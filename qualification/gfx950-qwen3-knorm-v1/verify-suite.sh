#!/usr/bin/env bash
set -euo pipefail
umask 077

if [[ $# != 9 || ($1 != prepare && $1 != run) ]]; then
  printf 'usage: %s prepare|run PROBE WORKER BUILD_DIR FERRIC_ROOT WEIGHTS_DIR GAMMA_DIR PRIVATE_DEVICE_ID EVIDENCE_DIR\n' "$0" >&2
  exit 2
fi
mode=$1
probe=$(realpath -- "$2")
worker=$(realpath -- "$3")
build=$(realpath -- "$4")
repo=$(realpath -- "$5")
weights=$(realpath -- "$6")
gamma=$(realpath -- "$7")
selector=$(realpath -- "$8")
evidence=$(realpath -m -- "$9")
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
producer_source="$repo/device/gfx950-qwen3-kproj-wave64-v1/src/lib.rs"
consumer_source="$repo/device/gfx950-qwen3-knorm-v1/src/lib.rs"
artifact="$evidence/artifact.json"
cases=(zero basis mixed cancellation epsilon)
identity=(--producer-object "$build/producer.hsaco" --producer-source "$producer_source"
  --consumer-object "$build/consumer.hsaco" --consumer-source "$consumer_source"
  --artifact "$artifact" --probe "$probe" --worker "$worker")
common=(--weights-dir "$weights" --gamma-dir "$gamma" "${identity[@]}")

if [[ $mode == prepare ]]; then
  mkdir -- "$evidence"
  "$probe" qwen3-knorm-chain-inspect \
    --producer-object "$build/producer.hsaco" --producer-source "$producer_source" \
    --consumer-object "$build/consumer.hsaco" --consumer-source "$consumer_source" \
    --metadata "$artifact"
  files=("$probe" "$worker" "$build/producer.hsaco" "$producer_source"
    "$build/consumer.hsaco" "$consumer_source" "$artifact"
    "$script_dir/reference.py" "$script_dir/verify-suite.sh"
    "$script_dir/../gfx950-qwen3-kproj-wave64-v1/reference.py"
    "$script_dir/../gfx950-qwen3-kproj-v1/reference.py"
    "$script_dir/../gfx950-qwen3-kproj-v1/extract_checkpoint.py"
    "$weights/weights.bf16le" "$weights/checkpoint.json" "$gamma/gamma.bf16le" "$gamma/gamma.json")
  for case_name in "${cases[@]}"; do
    mkdir -- "$evidence/$case_name"
    python3 -B "$script_dir/reference.py" generate "${common[@]}" \
      --case "$case_name" --case-dir "$evidence/$case_name/case" \
      > "$evidence/$case_name/generate.log"
    files+=("$evidence/$case_name/case/"*)
  done
  sha256sum -- "${files[@]}" > "$evidence/inputs.sha256"
  sha256sum --check --strict "$evidence/inputs.sha256" > "$evidence/prepared.txt"
  printf 'References frozen. No GPU dispatched. Recheck shared-host availability before run.\n'
  exit 0
fi

# The caller coordinates the shared GPU immediately before this phase. Any
# error is terminal; an existing run directory also prevents automatic retry.
sha256sum --check --strict "$evidence/inputs.sha256" > "$evidence/preflight.txt"
for case_name in "${cases[@]}"; do
  "$probe" qwen3-knorm-chain-run --worker "$worker" \
    --producer-object "$build/producer.hsaco" --producer-source "$producer_source" \
    --consumer-object "$build/consumer.hsaco" --consumer-source "$consumer_source" \
    --metadata "$artifact" --inputs "$evidence/$case_name/case/inputs.bf16le" \
    --weights "$weights/weights.bf16le" --norm-weights "$gamma/gamma.bf16le" \
    --device-id-file "$selector" --run-dir "$evidence/$case_name/run" \
    --allow-unauthenticated-machine-code
  python3 -B "$script_dir/reference.py" check "${common[@]}" \
    --case-dir "$evidence/$case_name/case" --run-dir "$evidence/$case_name/run" \
    --result "$evidence/$case_name/numerical.json" > "$evidence/$case_name/check.log"
done
sha256sum --check --strict "$evidence/inputs.sha256" > "$evidence/inputs-rechecked.txt"
