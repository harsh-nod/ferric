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

for tool in awk cargo cat chmod cmp cp git grep mkdir mktemp python3 rm sed sha256sum sort stat timeout wc; do
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

source_closure="$scratch/ferric-source-closure"
python3 -I "$repo/proofs/m1/evidence/measure-source-closure.py" \
    "$repo" "$source_closure" \
    >"$scratch/source-closure.transcript"
source_closure_digest=$(sha256sum "$source_closure" | awk '{ print $1 }')
mkdir -p "$output"

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
LC_ALL=C sort "$selected" -o "$selected"

commit=$(git -C "$repo" rev-parse --verify HEAD)
tree=$(git -C "$repo" rev-parse --verify 'HEAD^{tree}')
verus_digest=$(sha256sum "$verus_root/verus" | awk '{ print $1 }')
verus_closure_manifest_digest=$(
    sha256sum "$repo/proofs/verus/VERUS_CLOSURE_MANIFEST" | awk '{ print $1 }'
)
verus_closure_digest=$(
    sed -n 's/^closure-sha256=//p' "$repo/proofs/verus/VERUS_CLOSURE_MANIFEST"
)
registry_digest=$(sha256sum "$repo/proofs/m1/negative/REQUIRED_FOUNDATIONS" \
    | awk '{ print $1 }')
runner_digest=$(sha256sum "$repo/proofs/m1/negative/run-same-source.sh" \
    | awk '{ print $1 }')
{
    printf 'FORMAT=FERRIC-M1-NEGATIVE-RUN-V1\n'
    printf 'FERRIC_COMMIT=%s\n' "$commit"
    printf 'FERRIC_TREE=%s\n' "$tree"
    printf 'FERRIC_SOURCE_CLOSURE_SHA256=%s\n' "$source_closure_digest"
    printf 'VERUS_VERSION=%s\n' "$actual_version"
    printf 'VERUS_SHA256=%s\n' "$verus_digest"
    printf 'VERUS_CLOSURE_MANIFEST_SHA256=%s\n' "$verus_closure_manifest_digest"
    printf 'VERUS_CLOSURE_SHA256=%s\n' "$verus_closure_digest"
    printf 'REGISTRY_SHA256=%s\n' "$registry_digest"
    printf 'RUNNER_SHA256=%s\n' "$runner_digest"
    printf 'AUTHORITY=hostile-foundation-proof-rejection-only\n'
    printf 'NONCLAIM=no-m1-property-or-roadmap-closure\n'
} >"$output/RUN_IDENTITY"

copy_source() {
    destination=$1
    mkdir -p "$destination/proofs"
    cp -a "$repo/Cargo.toml" "$repo/Cargo.lock" "$repo/rust-toolchain.toml" \
        "$destination/"
    cp -a "$repo/adapters" "$destination/"
    cp -a "$repo/benches" "$destination/"
    cp -a "$repo/crates" "$destination/"
    cp -a "$repo/device" "$destination/"
    cp -a "$repo/proofs/m1" "$destination/proofs/"
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
    mutation_stdout="$scratch/$name.mutator-output"
    python3 -I "$mutation" "$copy" >"$mutation_stdout"
    {
        printf 'MUTATED_SOURCE=%s\n' "$source"
        printf 'MUTATION=%s\n' "$name"
        printf 'CLAUSE=%s\n' "$clause"
    } >"$scratch/expected-mutation"
    sed -n '1,3p' "$mutation_stdout" >"$scratch/actual-mutation-prefix"
    cmp -s "$scratch/expected-mutation" "$scratch/actual-mutation-prefix" || {
        printf 'FAIL: %s mutator attestation drifted\n' "$name" >&2
        exit 1
    }
    [ "$(wc -l <"$mutation_stdout")" -eq 4 ] \
        && sed -n '4p' "$mutation_stdout" \
            | grep -E '^ANCHOR_SHA256=[0-9a-f]{64}$' >/dev/null || {
        printf 'FAIL: %s mutator anchor identity drifted\n' "$name" >&2
        exit 1
    }
    {
        printf 'FORMAT=FERRIC-M1-NEGATIVE-MUTATION-V1\n'
        cat "$mutation_stdout"
        printf 'MUTATOR_SHA256=%s\n' "$(sha256sum "$mutation" | awk '{ print $1 }')"
        printf 'ORIGINAL_SOURCE_SHA256=%s\n' "$(sha256sum "$repo/$source" | awk '{ print $1 }')"
        printf 'MUTATED_SOURCE_SHA256=%s\n' "$(sha256sum "$copy/$source" | awk '{ print $1 }')"
        printf 'FOUNDATION=%s\n' "$foundation"
        printf 'OPEN_ASSURANCE_PROPERTY=%s\n' "$property"
        printf 'OPEN_PATH_OBLIGATION=%s\n' "$path_id"
        printf 'VERUS_PACKAGE=%s\n' "$package"
        printf 'VERUS_MODULE=%s\n' "$module"
        printf 'VERUS_FUNCTION=%s\n' "$function"
        printf 'EXPECTED_FAILURE_MARKER=%s\n' "$marker"
        printf 'CARGO_CHECK=passed\n'
    } >"$mutation_record"

    compile_transcript="$output/$name.compile.transcript"
    {
        printf 'FORMAT=FERRIC-M1-NEGATIVE-COMPILE-V1\n'
        printf 'MUTATION=%s\n' "$name"
        printf 'CARGO_PACKAGE=%s\n' "$package"
        printf 'COMMAND=cargo-check-locked-all-targets\n'
    } >"$compile_transcript"
    set +e
    (
        cd "$copy"
        CARGO_TERM_COLOR=never timeout "$timeout_seconds" cargo check \
            -p "$package" --locked --all-targets --target-dir "$compile_target"
    ) >>"$compile_transcript" 2>&1
    compile_status=$?
    set -e
    [ "$compile_status" -eq 0 ] || {
        printf 'FAIL: %s mutation did not compile (status %s)\n' \
            "$name" "$compile_status" >&2
        exit 1
    }

    transcript="$output/$name.verus.transcript"
    {
        printf 'FORMAT=FERRIC-M1-NEGATIVE-VERUS-V1\n'
        printf 'MUTATION=%s\n' "$name"
        printf 'VERUS_PACKAGE=%s\n' "$package"
        printf 'VERUS_MODULE=%s\n' "$module"
        printf 'VERUS_FUNCTION=%s\n' "$function"
        printf 'COMMAND=cargo-verus-build-lib-locked-release-no-cheating-exact-function\n'
        printf 'EXPECTED_FAILURE_MARKER=%s\n' "$marker"
    } >"$transcript"
    set +e
    (
        cd "$copy"
        VERUS_Z3_PATH="$verus_root/z3" CARGO_TERM_COLOR=never \
            timeout "$timeout_seconds" "$verus_root/cargo-verus" build \
                -p "$package" --locked --release --target-dir "$verus_target" \
                --fwd-verus-args-to roots -j 1 --lib -- --no-cheating \
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
    if [ "$proof_status" -ne 101 ]; then
        printf 'FAIL: %s returned unexpected cargo-verus status %s\n' \
            "$name" "$proof_status" >&2
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
    result="$output/$name.result"
    {
        printf 'FORMAT=FERRIC-M1-NEGATIVE-RESULT-V1\n'
        printf 'MUTATION=%s\n' "$name"
        printf 'RUN_IDENTITY_SHA256=%s\n' "$(sha256sum "$output/RUN_IDENTITY" | awk '{ print $1 }')"
        printf 'ACTIVE_FOUNDATIONS_SHA256=%s\n' "$(sha256sum "$active" | awk '{ print $1 }')"
        printf 'SELECTED_FOUNDATIONS_SHA256=%s\n' "$(sha256sum "$selected" | awk '{ print $1 }')"
        printf 'VERUS_CLOSURE_TRANSCRIPT_SHA256=%s\n' "$(sha256sum "$closure" | awk '{ print $1 }')"
        printf 'MUTATION_RECORD=%s.mutation\n' "$name"
        printf 'MUTATION_RECORD_SHA256=%s\n' "$(sha256sum "$mutation_record" | awk '{ print $1 }')"
        printf 'MUTATION_RECORD_SIZE=%s\n' "$(stat -c '%s' "$mutation_record")"
        printf 'COMPILE_TRANSCRIPT=%s.compile.transcript\n' "$name"
        printf 'COMPILE_TRANSCRIPT_SHA256=%s\n' "$(sha256sum "$compile_transcript" | awk '{ print $1 }')"
        printf 'COMPILE_TRANSCRIPT_SIZE=%s\n' "$(stat -c '%s' "$compile_transcript")"
        printf 'COMPILE_EXIT_STATUS=%s\n' "$compile_status"
        printf 'VERUS_TRANSCRIPT=%s.verus.transcript\n' "$name"
        printf 'VERUS_TRANSCRIPT_SHA256=%s\n' "$(sha256sum "$transcript" | awk '{ print $1 }')"
        printf 'VERUS_TRANSCRIPT_SIZE=%s\n' "$(stat -c '%s' "$transcript")"
        printf 'VERUS_EXIT_STATUS=%s\n' "$proof_status"
        printf 'RESULT=proof-rejected\n'
    } >"$result"
    printf 'PASS: %s compiled and pinned Verus rejected clause %s\n' "$name" "$clause"
    chmod -R u+w "$copy" 2>/dev/null || true
    rm -rf "$copy"
done <"$selected"
