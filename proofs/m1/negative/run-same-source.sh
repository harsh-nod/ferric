#!/bin/sh
set -eu

usage() {
    printf 'usage: %s REPO VERUS_ROOT OUTPUT_DIR [MUTATION ...]\n' "$0" >&2
    exit 2
}

[ "$#" -ge 3 ] || usage
repo=$(CDPATH='' cd -- "$1" && pwd)
verus_root=$(CDPATH='' cd -- "$2" && pwd)
output=$3
shift 3
[ ! -e "$output" ] && [ ! -L "$output" ] || {
    printf 'FAIL: M1 negative output already exists: %s\n' "$output" >&2
    exit 1
}

for tool in awk cargo cat chmod cmp cp git grep mkdir mktemp python3 rm sed sha256sum timeout; do
    command -v "$tool" >/dev/null 2>&1 || {
        printf 'FAIL: M1 negative runner requires %s\n' "$tool" >&2
        exit 1
    }
done
[ -x "$verus_root/cargo-verus" ] && [ -x "$verus_root/verus" ] \
    && [ -x "$verus_root/z3" ] || {
    printf 'FAIL: pinned cargo-verus, Verus, or Z3 is unavailable\n' >&2
    exit 1
}
[ -z "$(git -C "$repo" status --porcelain=v1 --untracked-files=all)" ] || {
    printf 'FAIL: M1 negative runner requires a clean source worktree\n' >&2
    exit 1
}

timeout_seconds=${FERRIC_M1_NEGATIVE_TIMEOUT_SECONDS:-600}
case "$timeout_seconds" in
    ''|*[!0-9]*) printf 'FAIL: invalid M1 negative timeout\n' >&2; exit 2 ;;
esac
[ "$timeout_seconds" -ge 1 ] && [ "$timeout_seconds" -le 1200 ] || {
    printf 'FAIL: M1 negative timeout must be 1 through 1200\n' >&2
    exit 2
}

mkdir -p "$output"
scratch=$(mktemp -d "${TMPDIR:-/tmp}/ferric-m1-negative.XXXXXX")
cleanup() {
    status=$?
    trap - EXIT HUP INT TERM
    chmod -R u+w "$scratch" 2>/dev/null || true
    rm -rf "$scratch" || true
    return "$status"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

closure="$output/verus-closure.transcript"
"$repo/proofs/verify-verus-closure.sh" \
    "$verus_root" "$repo/proofs/verus/VERUS_CLOSURE_MANIFEST" >"$closure" 2>&1 || {
    cat "$closure" >&2
    printf 'FAIL: pinned Verus closure authentication failed\n' >&2
    exit 1
}
expected_version=$(sed -n '1p' "$repo/proofs/verus/VERUS_VERSION")
actual_version=$(VERUS_Z3_PATH="$verus_root/z3" "$verus_root/verus" --version \
    | sed -n 's/^  Version: //p')
[ "$actual_version" = "$expected_version" ] || {
    printf 'FAIL: Verus version does not match the admitted release\n' >&2
    exit 1
}

active="$output/active-foundations"
python3 -I "$repo/proofs/m1/negative/check-registry.py" \
    "$repo" "$repo/proofs/m1/negative/REQUIRED_FOUNDATIONS" "$active"
selected="$output/selected-foundations"
if [ "$#" -eq 0 ]; then
    cp "$active" "$selected"
else
    : >"$selected"
    : >"$scratch/requested"
    for requested in "$@"; do
        case "$requested" in
            ''|*[!A-Za-z0-9_.-]*)
                printf 'FAIL: unsafe requested M1 mutation: %s\n' "$requested" >&2
                exit 2
                ;;
        esac
        if grep -Fx "$requested" "$scratch/requested" >/dev/null; then
            printf 'FAIL: duplicate requested M1 mutation: %s\n' "$requested" >&2
            exit 2
        fi
        printf '%s\n' "$requested" >>"$scratch/requested"
        row=$(awk -F '|' -v name="$requested" '$1 == name { print }' "$active")
        [ -n "$row" ] || {
            printf 'FAIL: unknown requested M1 mutation: %s\n' "$requested" >&2
            exit 2
        }
        [ "$(printf '%s\n' "$row" | awk 'END { print NR }')" -eq 1 ] || {
            printf 'FAIL: ambiguous requested M1 mutation: %s\n' "$requested" >&2
            exit 1
        }
        printf '%s\n' "$row" >>"$selected"
    done
fi

commit=$(git -C "$repo" rev-parse --verify HEAD)
tree=$(git -C "$repo" rev-parse --verify 'HEAD^{tree}')
verus_digest=$(sha256sum "$verus_root/verus" | awk '{ print $1 }')
registry_digest=$(sha256sum "$repo/proofs/m1/negative/REQUIRED_FOUNDATIONS" \
    | awk '{ print $1 }')
{
    printf 'FORMAT=FERRIC-M1-NEGATIVE-RUN-V1\n'
    printf 'FERRIC_COMMIT=%s\n' "$commit"
    printf 'FERRIC_TREE=%s\n' "$tree"
    printf 'VERUS_VERSION=%s\n' "$actual_version"
    printf 'VERUS_SHA256=%s\n' "$verus_digest"
    printf 'REGISTRY_SHA256=%s\n' "$registry_digest"
    printf 'AUTHORITY=hostile-foundation-proof-rejection-only\n'
    printf 'NONCLAIM=no-m1-property-or-roadmap-closure\n'
} >"$output/RUN_IDENTITY"

copy_source() {
    destination=$1
    mkdir -p "$destination"
    cp -a "$repo/Cargo.toml" "$repo/Cargo.lock" "$repo/rust-toolchain.toml" \
        "$destination/"
    cp -a "$repo/crates" "$destination/"
    chmod -R u+w "$destination"
}

compile_target="$scratch/compile-target"
verus_target="$scratch/verus-target"
while IFS='|' read -r name foundation property path_id package source mutator marker module function clause extra; do
    [ -n "$name" ] && [ -n "$foundation" ] && [ -n "$property" ] \
        && [ -n "$path_id" ] && [ -n "$package" ] && [ -n "$source" ] \
        && [ -n "$mutator" ] && [ -n "$marker" ] && [ -n "$module" ] \
        && [ -n "$function" ] && [ -n "$clause" ] && [ -z "$extra" ] || {
        printf 'FAIL: malformed selected M1 foundation mutation\n' >&2
        exit 1
    }
    copy="$scratch/copy-$name"
    copy_source "$copy"
    mutation="$repo/proofs/m1/negative/components/$mutator"
    mutation_record="$output/$name.mutation"
    python3 -I "$mutation" "$copy" >"$mutation_record"
    {
        printf 'MUTATED_SOURCE=%s\n' "$source"
        printf 'MUTATION=%s\n' "$name"
        printf 'CLAUSE=%s\n' "$clause"
    } >"$scratch/expected-mutation"
    cmp -s "$scratch/expected-mutation" "$mutation_record" || {
        printf 'FAIL: %s mutator attestation drifted\n' "$name" >&2
        exit 1
    }
    {
        printf 'MUTATOR_SHA256=%s\n' "$(sha256sum "$mutation" | awk '{ print $1 }')"
        printf 'MUTATED_SOURCE_SHA256=%s\n' "$(sha256sum "$copy/$source" | awk '{ print $1 }')"
        printf 'FOUNDATION=%s\n' "$foundation"
        printf 'OPEN_ASSURANCE_PROPERTY=%s\n' "$property"
        printf 'OPEN_PATH_OBLIGATION=%s\n' "$path_id"
        printf 'VERUS_PACKAGE=%s\n' "$package"
        printf 'VERUS_MODULE=%s\n' "$module"
        printf 'VERUS_FUNCTION=%s\n' "$function"
        printf 'EXPECTED_FAILURE_MARKER=%s\n' "$marker"
    } >>"$mutation_record"

    compile_transcript="$output/$name.compile.transcript"
    set +e
    (
        cd "$copy"
        CARGO_TERM_COLOR=never timeout "$timeout_seconds" cargo check \
            -p "$package" --locked --all-targets --target-dir "$compile_target"
    ) >"$compile_transcript" 2>&1
    compile_status=$?
    set -e
    [ "$compile_status" -eq 0 ] || {
        printf 'FAIL: %s mutation did not compile (status %s)\n' \
            "$name" "$compile_status" >&2
        exit 1
    }
    printf 'CARGO_CHECK=passed\n' >>"$mutation_record"

    transcript="$output/$name.verus.transcript"
    {
        printf 'VERUS_PACKAGE=%s\n' "$package"
        printf 'VERUS_MODULE=%s\n' "$module"
        printf 'VERUS_FUNCTION=%s\n' "$function"
    } >"$transcript"
    set +e
    (
        cd "$copy"
        VERUS_Z3_PATH="$verus_root/z3" CARGO_TERM_COLOR=never \
            timeout "$timeout_seconds" "$verus_root/cargo-verus" build \
                -p "$package" --locked --release --target-dir "$verus_target" \
                --fwd-verus-args-to roots -j 1 -- --no-cheating \
                --verify-only-module "$module" --verify-function "$function"
    ) >>"$transcript" 2>&1
    proof_status=$?
    set -e
    if [ "$proof_status" -eq 0 ]; then
        printf 'FAIL: %s actual-body mutation was accepted\n' "$name" >&2
        exit 1
    fi
    if [ "$proof_status" -eq 124 ]; then
        printf 'FAIL: %s actual-body mutation timed out\n' "$name" >&2
        exit 1
    fi
    case "$marker" in
        assertion)
            grep -F 'assertion failed' "$transcript" >/dev/null || {
                printf 'FAIL: %s did not fail its expected assertion\n' "$name" >&2
                exit 1
            }
            ;;
        postcondition)
            grep -F 'postcondition not satisfied' "$transcript" >/dev/null || {
                printf 'FAIL: %s did not fail its expected postcondition\n' "$name" >&2
                exit 1
            }
            ;;
        *)
            printf 'FAIL: unknown M1 proof-failure marker: %s\n' "$marker" >&2
            exit 1
            ;;
    esac
    printf 'PASS: %s compiled and pinned Verus rejected clause %s\n' "$name" "$clause"
    chmod -R u+w "$copy" 2>/dev/null || true
    rm -rf "$copy"
done <"$selected"
