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

for tool in awk cargo cat chmod cp env grep kill mkdir mktemp mv python3 rm setsid sha256sum sleep sort timeout; do
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
[ "$timeout_seconds" -ge 1 ] && [ "$timeout_seconds" -le 3600 ] || {
    printf 'FAIL: negative-test timeout must be 1 through 3600\n' >&2
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
    mkdir -p "$destination/proofs"
    cp -a "$repo/Cargo.toml" "$repo/Cargo.lock" "$repo/rust-toolchain.toml" "$destination/"
    cp -a "$repo/benches" "$repo/crates" "$destination/"
    cp -a "$repo/proofs/m1" "$destination/proofs/"
    chmod -R u+w "$destination"
}

clean_package() {
    copy=$1
    package=$2
    target=$3
    transcript=$4
    env CARGO_TERM_COLOR=never cargo clean \
        --manifest-path "$copy/Cargo.toml" \
        -p "$package" --locked --release --target-dir "$target" \
        >>"$transcript" 2>&1
}

run_rejected() {
    name=$1
    copy=$2
    package=$3
    module=$4
    function=$5
    target=$6
    transcript="$output/${name}.transcript"
    {
        printf 'FORMAT=FERRIC-NEGATIVE-REJECTION-V1\n'
        printf 'VERUS_PACKAGE=%s\n' "$package"
        printf 'VERUS_MODULE=%s\n' "$module"
        printf 'VERUS_FUNCTION=%s\n' "$function"
        printf 'CACHE_LANE=%s\n' "$package"
        printf 'ROOT_INVALIDATION=cargo-clean-package-release-before-and-after-build\n'
        printf 'PHASE=pre-mutation-root-clean\n'
    } >"$transcript"
    clean_package "$copy" "$package" "$target" "$transcript" || {
        printf 'FAIL: %s pre-mutation package clean failed\n' "$name" >&2
        exit 1
    }
    set +e
    setsid sh -c 'cd "$1"; shift; exec "$@"' \
        ferric-negative-child "$copy" \
        env VERUS_Z3_PATH="$verus_root/z3" CARGO_TERM_COLOR=never \
            timeout --kill-after=10 "$timeout_seconds" \
            "$verus_root/cargo-verus" build \
                -p "$package" --locked --release --target-dir "$target" \
                --fwd-verus-args-to roots -j 1 --lib -- --no-cheating \
                --verify-only-module "$module" --verify-function "$function" \
        >>"$transcript" 2>&1 &
    component_child_pid=$!
    wait "$component_child_pid"
    status=$?
    component_child_pid=
    set -e
    printf 'PHASE=post-mutation-root-clean\n' >>"$transcript"
    clean_package "$copy" "$package" "$target" "$transcript" || {
        printf 'FAIL: %s post-mutation package clean failed\n' "$name" >&2
        exit 1
    }
    if [ "$status" -eq 0 ]; then
        printf 'FAIL: %s mutation was accepted\n' "$name" >&2
        exit 1
    fi
    if [ "$status" -eq 124 ]; then
        printf 'FAIL: %s mutation timed out\n' "$name" >&2
        exit 1
    fi
    if [ "$status" -ne 101 ]; then
        printf 'FAIL: %s mutation returned unexpected status %s\n' "$name" "$status" >&2
        exit 1
    fi
    grep -F "Compiling $package " "$transcript" >/dev/null || {
        printf 'FAIL: %s did not recompile its mutated package root\n' "$name" >&2
        exit 1
    }
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
    copy=$7
    target=$8
    mutation="$repo/proofs/negative/components/$mutator"
    copy_source "$copy"
    mutation_marker="$output/$name.mutation"
    python3 -I "$mutation" "$copy" >"$mutation_marker"
    python3 -I "$repo/proofs/negative/check-mutation.py" \
        "$repo" "$copy" "$mutation_marker" "$package"
    printf 'MUTATOR_SHA256=%s\n' "$(sha256sum "$mutation" | awk '{ print $1 }')" \
        >>"$mutation_marker"
    printf 'VERUS_PACKAGE=%s\n' "$package" >>"$mutation_marker"
    printf 'VERUS_MODULE=%s\n' "$module" >>"$mutation_marker"
    printf 'VERUS_FUNCTION=%s\n' "$function" >>"$mutation_marker"
    run_rejected "$name" "$copy" "$package" "$module" "$function" "$target"
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
    chmod -R u+w "$copy" 2>/dev/null || true
    rm -rf "$copy"
}

run_package() {
    package=$1
    components=$2
    copy="$scratch/sources/$package"
    target="$scratch/targets/$package"
    component_child_pid=
    terminate_component_child() {
        [ -n "$component_child_pid" ] || return 0
        pid=$component_child_pid
        component_child_pid=
        child_found=false
        if kill -0 "-$pid" 2>/dev/null; then
            kill -TERM "-$pid" 2>/dev/null || true
            child_found=true
        elif kill -0 "$pid" 2>/dev/null; then
            kill -TERM "$pid" 2>/dev/null || true
            child_found=true
        fi
        if [ "$child_found" = true ]; then
            sleep 1
            kill -KILL "-$pid" 2>/dev/null || true
            kill -KILL "$pid" 2>/dev/null || true
        fi
        wait "$pid" 2>/dev/null || true
    }
    cleanup_package_worker() {
        status=$?
        trap - EXIT HUP INT TERM
        terminate_component_child
        chmod -R u+w "$copy" 2>/dev/null || true
        rm -rf "$copy" || true
        if [ "$status" -ne 0 ]; then
            printf '%s\n' "$status" >"$scratch/failures/$package.tmp"
            mv "$scratch/failures/$package.tmp" "$scratch/failures/$package"
        fi
        return "$status"
    }
    trap cleanup_package_worker EXIT
    trap 'exit 129' HUP
    trap 'exit 130' INT
    trap 'exit 143' TERM
    while IFS='|' read -r name row_package mutator marker module function extra; do
        [ "$row_package" = "$package" ] && [ -z "$extra" ] || {
            printf 'FAIL: malformed %s negative cache lane\n' "$package" >&2
            exit 1
        }
        run_component "$name" "$package" "$mutator" "$marker" "$module" "$function" \
            "$copy" "$target"
    done <"$components"
}

wait_package_batch() {
    while :; do
        failed_package=
        all_finished=true
        while IFS='|' read -r package pid extra; do
            [ -n "$package" ] && [ -n "$pid" ] && [ -z "$extra" ] || {
                printf 'FAIL: malformed negative-test cache-lane record\n' >&2
                exit 1
            }
            if [ -f "$scratch/failures/$package" ]; then
                failed_package=$package
                break
            fi
            kill -0 "$pid" 2>/dev/null && all_finished=false
        done <"$scratch/pids"
        if [ -n "$failed_package" ]; then
            while IFS='|' read -r _package pid _extra; do
                kill -TERM "$pid" 2>/dev/null || true
            done <"$scratch/pids"
            while IFS='|' read -r _package pid _extra; do
                wait "$pid" 2>/dev/null || true
            done <"$scratch/pids"
            printf 'FAIL: negative cache lane %s failed (status %s)\n' \
                "$failed_package" "$(cat "$scratch/failures/$failed_package")" >&2
            : >"$scratch/pids"
            exit 1
        fi
        [ "$all_finished" = true ] && break
        sleep 1
    done
    while IFS='|' read -r package pid extra; do
        set +e
        wait "$pid"
        status=$?
        set -e
        if [ "$status" -ne 0 ]; then
            printf 'FAIL: negative cache lane %s failed (status %s)\n' \
                "$package" "$status" >&2
            : >"$scratch/pids"
            exit 1
        fi
    done <"$scratch/pids"
    : >"$scratch/pids"
    running=0
}

mkdir -p "$scratch/sources" "$scratch/targets" "$scratch/results" \
    "$scratch/failures" "$scratch/package-components"
: >"$scratch/pids"
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
    case "$package" in
        ferric-spec|ferric-engine) ;;
        *) printf 'FAIL: unsupported negative mutation package: %s\n' "$package" >&2; exit 1 ;;
    esac
    printf '%s|%s|%s|%s|%s|%s\n' \
        "$name" "$package" "$mutator" "$marker" "$module" "$function" \
        >>"$scratch/package-components/$package"
done <"$active"

awk -F '|' '!seen[$2]++ { print $2 }' "$active" >"$scratch/packages"
running=0
while IFS= read -r package; do
    run_package "$package" "$scratch/package-components/$package" &
    pid=$!
    printf '%s|%s\n' "$package" "$pid" >>"$scratch/pids"
    running=$((running + 1))
    if [ "$running" -eq "$parallel_jobs" ]; then
        wait_package_batch
    fi
done <"$scratch/packages"
[ "$running" -eq 0 ] || wait_package_batch
while IFS='|' read -r name _rest; do
    cat "$scratch/results/$name"
done <"$active"
