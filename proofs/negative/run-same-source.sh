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

for tool in awk chmod cp grep mktemp python3 rm sed sha256sum timeout; do
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

scratch=$(mktemp -d "${TMPDIR:-/tmp}/ferric-negative.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

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
    target=$(mktemp -d "$scratch/${name}-target.XXXXXX")
    transcript="$output/${name}.transcript"
    set +e
    (
        cd "$copy"
        VERUS_Z3_PATH="$verus_root/z3" \
            CARGO_TERM_COLOR=never \
            timeout "$timeout_seconds" "$verus_root/cargo-verus" build \
                -p "$package" --locked --release --target-dir "$target" \
                --fwd-verus-args-to roots -j 1 -- --no-cheating
    ) >"$transcript" 2>&1
    status=$?
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
[ "$(sed -n '1p' "$registry")" = 'format=FERRIC-NEGATIVE-COMPONENTS-V1' ] || {
    printf 'FAIL: unsupported negative component registry\n' >&2
    exit 1
}
active="$scratch/active-components"
: >"$active"
while IFS= read -r record; do
    case "$record" in
        format=*) continue ;;
        always=*) printf '%s\n' "${record#always=}" >>"$active" ;;
        when-verus=*)
            fields=${record#when-verus=}
            source=${fields%%|*}
            rest=${fields#*|}
            [ "$rest" != "$fields" ] || {
                printf 'FAIL: malformed conditional negative component\n' >&2
                exit 1
            }
            if grep -Eq 'verus[[:space:]]*![[:space:]]*\{' "$repo/$source"; then
                printf '%s\n' "$rest" >>"$active"
            fi
            ;;
        *) printf 'FAIL: malformed negative component record: %s\n' "$record" >&2; exit 1 ;;
    esac
done <"$registry"
[ -s "$active" ] || {
    printf 'FAIL: negative component registry selected no mutations\n' >&2
    exit 1
}

while IFS='|' read -r name package mutator marker extra; do
    [ -n "$name" ] && [ -n "$package" ] && [ -n "$mutator" ] && [ -n "$marker" ] && [ -z "$extra" ] || {
        printf 'FAIL: malformed active negative component\n' >&2
        exit 1
    }
    case "$name$package$mutator" in
        *[!A-Za-z0-9_.-]*) printf 'FAIL: unsafe negative component identity\n' >&2; exit 1 ;;
    esac
    mutation="$repo/proofs/negative/components/$mutator"
    [ -f "$mutation" ] || {
        printf 'FAIL: required %s actual-body mutation is missing: %s\n' "$name" "$mutator" >&2
        exit 1
    }
    copy="$scratch/$name"
    copy_source "$copy"
    mutation_marker="$output/$name.mutation"
    python3 "$mutation" "$copy" >"$mutation_marker"
    grep -F 'MUTATED_SOURCE=' "$mutation_marker" >/dev/null || {
        printf 'FAIL: %s mutator did not attest source mutation\n' "$name" >&2
        exit 1
    }
    printf 'MUTATOR_SHA256=%s\n' "$(sha256sum "$mutation" | awk '{ print $1 }')" \
        >>"$mutation_marker"
    run_rejected "$name" "$copy" "$package"
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
    printf 'PASS: %s actual-body mutation rejected (%s)\n' "$name" "$marker"
done <"$active"
