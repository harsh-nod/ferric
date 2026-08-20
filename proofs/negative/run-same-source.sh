#!/bin/sh
set -eu

usage() {
    printf 'usage: %s REPO VERUS_ROOT OUTPUT_DIR\n' "$0" >&2
    exit 2
}

[ "$#" -eq 3 ] || usage
repo=$(CDPATH='' cd -- "$1" && pwd)
verus_root=$(CDPATH='' cd -- "$2" && pwd)
output=$3
[ ! -e "$output" ] || {
    printf 'FAIL: negative-test output already exists: %s\n' "$output" >&2
    exit 1
}
mkdir -p "$output"

for tool in awk cat chmod cp grep mktemp python3 rm sha256sum timeout; do
    command -v "$tool" >/dev/null 2>&1 || {
        printf 'FAIL: negative tests require %s\n' "$tool" >&2
        exit 1
    }
done
[ -x "$verus_root/cargo-verus" ] && [ -x "$verus_root/z3" ] || {
    printf 'FAIL: authenticated cargo-verus or Z3 is unavailable\n' >&2
    exit 1
}

timeout_seconds=${FERRIC_NEGATIVE_TIMEOUT_SECONDS:-300}
case "$timeout_seconds" in
    ''|*[!0-9]*) printf 'FAIL: invalid negative-test timeout\n' >&2; exit 2 ;;
esac
[ "$timeout_seconds" -ge 1 ] && [ "$timeout_seconds" -le 1200 ] || {
    printf 'FAIL: negative-test timeout must be 1 through 1200\n' >&2
    exit 2
}
parallel_jobs=${FERRIC_NEGATIVE_JOBS:-2}
case "$parallel_jobs" in
    ''|*[!0-9]*) printf 'FAIL: invalid negative-test parallelism\n' >&2; exit 2 ;;
esac
[ "$parallel_jobs" -ge 1 ] && [ "$parallel_jobs" -le 8 ] || {
    printf 'FAIL: negative-test parallelism must be 1 through 8\n' >&2
    exit 2
}

scratch=$(mktemp -d "${TMPDIR:-/tmp}/ferric-negative.XXXXXX")
cleanup_main() {
    status=$?
    trap - EXIT HUP INT TERM
    if [ -f "$scratch/pids" ]; then
        while IFS='|' read -r _name pid _extra; do
            [ -n "$pid" ] && kill -TERM "$pid" 2>/dev/null || true
        done <"$scratch/pids"
        while IFS='|' read -r _name pid _extra; do
            [ -n "$pid" ] && wait "$pid" 2>/dev/null || true
        done <"$scratch/pids"
    fi
    rm -rf "$scratch" || true
    return "$status"
}
trap cleanup_main EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

copy_source() {
    destination=$1
    mkdir -p "$destination"
    cp -a "$repo/Cargo.toml" "$repo/Cargo.lock" "$repo/rust-toolchain.toml" "$destination/"
    cp -a "$repo/crates" "$destination/"
    chmod -R u+w "$destination"
}

run_rejected() {
    name=$1
    copy=$2
    package=$3
    module=$4
    function=$5
    target="$scratch/targets/$name"
    mkdir -p "$target"
    transcript="$output/${name}.transcript"
    {
        printf 'VERUS_PACKAGE=%s\n' "$package"
        printf 'VERUS_MODULE=%s\n' "$module"
        printf 'VERUS_FUNCTION=%s\n' "$function"
    } >"$transcript"
    set +e
    (
        cd "$copy"
        VERUS_Z3_PATH="$verus_root/z3" \
            CARGO_TERM_COLOR=never \
            timeout "$timeout_seconds" "$verus_root/cargo-verus" build \
                -p "$package" --locked --release --target-dir "$target" \
                --fwd-verus-args-to roots -j 1 -- --no-cheating \
                --verify-only-module "$module" --verify-function "$function"
    ) >>"$transcript" 2>&1 &
    component_child_pid=$!
    wait "$component_child_pid"
    status=$?
    component_child_pid=
    set -e
    if [ "$status" -eq 0 ]; then
        printf 'FAIL: %s mutation was accepted\n' "$name" >&2
        exit 1
    fi
    if [ "$status" -eq 124 ]; then
        printf 'FAIL: %s mutation timed out\n' "$name" >&2
        exit 1
    fi
}

registry="$repo/proofs/negative/REQUIRED_COMPONENTS"
[ -f "$registry" ] || {
    printf 'FAIL: negative component registry is unavailable\n' >&2
    exit 1
}
active="$scratch/active-components"
python3 -I "$repo/proofs/negative/check-registry.py" "$repo" "$registry" "$active"

run_component() {
    name=$1
    package=$2
    mutator=$3
    marker=$4
    module=$5
    function=$6
    mutation="$repo/proofs/negative/components/$mutator"
    copy="$scratch/copies/$name"
    target="$scratch/targets/$name"
    component_child_pid=
    cleanup_component() {
        status=$?
        trap - EXIT HUP INT TERM
        if [ -n "$component_child_pid" ]; then
            kill -TERM -"$component_child_pid" 2>/dev/null || true
            wait "$component_child_pid" 2>/dev/null || true
        fi
        chmod -R u+w "$copy" "$target" 2>/dev/null || true
        rm -rf "$copy" "$target" || true
        return "$status"
    }
    trap cleanup_component EXIT
    trap 'exit 129' HUP
    trap 'exit 130' INT
    trap 'exit 143' TERM
    copy_source "$copy"
    mutation_marker="$output/$name.mutation"
    python3 -I "$mutation" "$copy" >"$mutation_marker"
    grep -F 'MUTATED_SOURCE=' "$mutation_marker" >/dev/null || {
        printf 'FAIL: %s mutator did not attest source mutation\n' "$name" >&2
        exit 1
    }
    printf 'MUTATOR_SHA256=%s\n' "$(sha256sum "$mutation" | awk '{ print $1 }')" \
        >>"$mutation_marker"
    printf 'VERUS_PACKAGE=%s\n' "$package" >>"$mutation_marker"
    printf 'VERUS_MODULE=%s\n' "$module" >>"$mutation_marker"
    printf 'VERUS_FUNCTION=%s\n' "$function" >>"$mutation_marker"
    run_rejected "$name" "$copy" "$package" "$module" "$function"
    case "$marker" in
        proof)
            grep -Eq 'verification results:: [0-9]+ verified, [1-9][0-9]* errors|postcondition not satisfied|assertion failed' \
                "$output/$name.transcript" || {
                printf 'FAIL: %s did not fail a proof obligation\n' "$name" >&2
                exit 1
            }
            ;;
        no-cheating)
            grep -F 'assume/admit not allowed with --no-cheating' "$output/$name.transcript" >/dev/null || {
                printf 'FAIL: %s did not fail at the no-cheating boundary\n' "$name" >&2
                exit 1
            }
            ;;
        *) printf 'FAIL: unknown negative failure marker: %s\n' "$marker" >&2; exit 1 ;;
    esac
    printf 'PASS: %s actual-body mutation rejected (%s)\n' "$name" "$marker" \
        >"$scratch/results/$name"
}

wait_batch() {
    while IFS='|' read -r name pid extra; do
        [ -n "$name" ] && [ -n "$pid" ] && [ -z "$extra" ] || {
            printf 'FAIL: malformed negative-test job record\n' >&2
            exit 1
        }
        set +e
        wait "$pid"
        status=$?
        set -e
        if [ "$status" -ne 0 ]; then
            printf '%s|%s\n' "$name" "$status" >>"$scratch/failures"
        fi
    done <"$scratch/pids"
    : >"$scratch/pids"
    running=0
}

mkdir -p "$scratch/copies" "$scratch/targets" "$scratch/results"
: >"$scratch/pids"
: >"$scratch/failures"
running=0
while IFS='|' read -r name package mutator marker module function extra; do
    [ -n "$name" ] && [ -n "$package" ] && [ -n "$mutator" ] && [ -n "$marker" ] \
        && [ -n "$module" ] && [ -n "$function" ] && [ -z "$extra" ] || {
        printf 'FAIL: malformed active negative component\n' >&2
        exit 1
    }
    [ -f "$repo/proofs/negative/components/$mutator" ] || {
        printf 'FAIL: required %s actual-body mutation is missing: %s\n' "$name" "$mutator" >&2
        exit 1
    }
    run_component "$name" "$package" "$mutator" "$marker" "$module" "$function" &
    pid=$!
    printf '%s|%s\n' "$name" "$pid" >>"$scratch/pids"
    running=$((running + 1))
    if [ "$running" -eq "$parallel_jobs" ]; then
        wait_batch
    fi
done <"$active"
[ "$running" -eq 0 ] || wait_batch

if [ -s "$scratch/failures" ]; then
    printf 'FAIL: negative mutation jobs failed:\n' >&2
    while IFS='|' read -r name status; do
        printf '  %s (status %s)\n' "$name" "$status" >&2
    done <"$scratch/failures"
    exit 1
fi
while IFS='|' read -r name _rest; do
    cat "$scratch/results/$name"
done <"$active"
