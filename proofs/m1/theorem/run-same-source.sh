#!/bin/sh
set -eu

usage() {
    printf 'usage: %s REPO VERUS_ROOT OUTPUT_DIR [THEOREM ...]\n' "$0" >&2
    exit 2
}

[ "$#" -ge 3 ] || usage
repo=$(CDPATH='' cd -- "$1" && pwd)
verus_root=$(CDPATH='' cd -- "$2" && pwd)
output=$3
shift 3
[ ! -e "$output" ] && [ ! -L "$output" ] || {
    printf 'FAIL: M1 theorem output already exists: %s\n' "$output" >&2
    exit 1
}
for tool in awk cargo cat chmod cp env git grep kill mkdir mktemp python3 rm sed setsid sha256sum sleep sort stat timeout tr wc; do
    command -v "$tool" >/dev/null 2>&1 || {
        printf 'FAIL: M1 theorem runner requires %s\n' "$tool" >&2
        exit 1
    }
done
[ -x "$verus_root/cargo-verus" ] && [ -x "$verus_root/verus" ] \
    && [ -x "$verus_root/z3" ] || {
    printf 'FAIL: pinned cargo-verus, Verus, or Z3 is unavailable\n' >&2
    exit 1
}
[ -z "$(git -C "$repo" status --porcelain=v1 --untracked-files=all)" ] || {
    printf 'FAIL: M1 theorem runner requires a clean source worktree\n' >&2
    exit 1
}

timeout_seconds=${FERRIC_M1_THEOREM_TIMEOUT_SECONDS:-600}
case "$timeout_seconds" in
    ''|*[!0-9]*) printf 'FAIL: invalid M1 theorem timeout\n' >&2; exit 2 ;;
esac
[ "$timeout_seconds" -ge 1 ] && [ "$timeout_seconds" -le 1200 ] || {
    printf 'FAIL: M1 theorem timeout must be 1 through 1200\n' >&2
    exit 2
}

scratch=$(mktemp -d "${TMPDIR:-/tmp}/ferric-m1-theorem.XXXXXX")
active_child_pid=
terminate_active_child() {
    [ -n "$active_child_pid" ] || return 0
    pid=$active_child_pid
    active_child_pid=
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
cleanup() {
    status=$?
    trap - EXIT HUP INT TERM
    terminate_active_child
    chmod -R u+w "$scratch" 2>/dev/null || true
    rm -rf "$scratch" || true
    return "$status"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

run_child() {
    child_transcript=$1
    child_directory=$2
    shift 2
    setsid sh -c 'cd "$1"; shift; exec "$@"' \
        ferric-m1-theorem-child "$child_directory" "$@" \
        >>"$child_transcript" 2>&1 &
    active_child_pid=$!
    set +e
    wait "$active_child_pid"
    child_status=$?
    set -e
    terminate_active_child
}

source_closure="$scratch/ferric-source-closure"
python3 -I "$repo/proofs/m1/evidence/measure-source-closure.py" \
    "$repo" "$source_closure" >"$scratch/source-closure.transcript"
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
toolchain=$(sed -n 's/^channel = "\([^"]*\)"/\1/p' "$repo/rust-toolchain.toml")
[ -n "$toolchain" ] || {
    printf 'FAIL: Rust toolchain identity is unavailable\n' >&2
    exit 1
}
expected_toolchain="${toolchain}-x86_64-unknown-linux-gnu"

active="$output/active-foundations"
python3 -I "$repo/proofs/m1/theorem/check-registry.py" \
    "$repo" "$repo/proofs/m1/theorem/REQUIRED_FOUNDATIONS" "$active"
selected="$output/selected-foundations"
if [ "$#" -eq 0 ]; then
    cp "$active" "$selected"
else
    : >"$selected"
    : >"$scratch/requested"
    for requested in "$@"; do
        case "$requested" in
            ''|*[!A-Za-z0-9_.-]*)
                printf 'FAIL: unsafe requested M1 theorem: %s\n' "$requested" >&2
                exit 2
                ;;
        esac
        if grep -Fx "$requested" "$scratch/requested" >/dev/null; then
            printf 'FAIL: duplicate requested M1 theorem: %s\n' "$requested" >&2
            exit 2
        fi
        printf '%s\n' "$requested" >>"$scratch/requested"
        row=$(awk -F '|' -v name="$requested" '$1 == name { print }' "$active")
        [ -n "$row" ] || {
            printf 'FAIL: unknown requested M1 theorem: %s\n' "$requested" >&2
            exit 2
        }
        [ "$(printf '%s\n' "$row" | awk 'END { print NR }')" -eq 1 ] || {
            printf 'FAIL: ambiguous requested M1 theorem: %s\n' "$requested" >&2
            exit 1
        }
        printf '%s\n' "$row" >>"$selected"
    done
fi
LC_ALL=C sort "$selected" -o "$selected"

commit=$(git -C "$repo" rev-parse --verify HEAD)
tree=$(git -C "$repo" rev-parse --verify 'HEAD^{tree}')
verus_digest=$(sha256sum "$verus_root/verus" | awk '{ print $1 }')
manifest_digest=$(sha256sum "$repo/proofs/verus/VERUS_CLOSURE_MANIFEST" | awk '{ print $1 }')
verus_closure_digest=$(sed -n 's/^closure-sha256=//p' "$repo/proofs/verus/VERUS_CLOSURE_MANIFEST")
registry_digest=$(sha256sum "$repo/proofs/m1/theorem/REQUIRED_FOUNDATIONS" | awk '{ print $1 }')
runner_digest=$(sha256sum "$repo/proofs/m1/theorem/run-same-source.sh" | awk '{ print $1 }')
coverage_digest=$(sha256sum "$repo/proofs/VERIFIED_MODULES" | awk '{ print $1 }')
{
    printf 'FORMAT=FERRIC-M1-POSITIVE-RUN-V1\n'
    printf 'FERRIC_COMMIT=%s\n' "$commit"
    printf 'FERRIC_TREE=%s\n' "$tree"
    printf 'FERRIC_SOURCE_CLOSURE_SHA256=%s\n' "$source_closure_digest"
    printf 'VERUS_VERSION=%s\n' "$actual_version"
    printf 'VERUS_SHA256=%s\n' "$verus_digest"
    printf 'VERUS_CLOSURE_MANIFEST_SHA256=%s\n' "$manifest_digest"
    printf 'VERUS_CLOSURE_SHA256=%s\n' "$verus_closure_digest"
    printf 'VERIFIED_MODULES_SHA256=%s\n' "$coverage_digest"
    printf 'REGISTRY_SHA256=%s\n' "$registry_digest"
    printf 'RUNNER_SHA256=%s\n' "$runner_digest"
    printf 'AUTHORITY=direct-verus-foundation-success-only\n'
    printf 'NONCLAIM=no-m1-property-path-or-roadmap-closure\n'
} >"$output/RUN_IDENTITY"

compile_transcript="$output/ferric-spec.compile.transcript"
{
    printf 'FORMAT=FERRIC-M1-POSITIVE-COMPILE-V1\n'
    printf 'CARGO_PACKAGE=ferric-spec\n'
    printf 'COMMAND=cargo-check-locked-all-targets\n'
} >"$compile_transcript"
run_child "$compile_transcript" "$repo" env CARGO_TERM_COLOR=never \
    timeout "$timeout_seconds" cargo check -p ferric-spec --locked --all-targets \
    --target-dir "$scratch/compile-target"
compile_status=$child_status
[ "$compile_status" -eq 0 ] || {
    printf 'FAIL: M1 theorem source did not compile (status %s)\n' "$compile_status" >&2
    exit 1
}

while IFS='|' read -r name foundation property path_id package source module function extra; do
    [ -n "$name" ] && [ -n "$foundation" ] && [ -n "$property" ] \
        && [ -n "$path_id" ] && [ -n "$package" ] && [ -n "$source" ] \
        && [ -n "$module" ] && [ -n "$function" ] && [ -z "$extra" ] || {
        printf 'FAIL: malformed selected M1 positive theorem\n' >&2
        exit 1
    }
    [ "$package" = ferric-spec ] || {
        printf 'FAIL: unsupported M1 theorem package: %s\n' "$package" >&2
        exit 1
    }
    crate_name=$(printf '%s' "$package" | tr '-' '_')
    compiler_path="${crate_name}::${module}::${function}"
    source_digest=$(sha256sum "$repo/$source" | awk '{ print $1 }')
    function_source_identity=$(
        printf 'FERRIC-M1-THEOREM-SOURCE-IDENTITY-V1|%s|%s\n' \
            "$source_digest" "$compiler_path" | sha256sum | awk '{ print $1 }'
    )

    transcript="$output/$name.verus.transcript"
    {
        printf 'FORMAT=FERRIC-M1-POSITIVE-VERUS-V1\n'
        printf 'THEOREM=%s\n' "$name"
        printf 'VERUS_PACKAGE=%s\n' "$package"
        printf 'VERUS_MODULE=%s\n' "$module"
        printf 'VERUS_FUNCTION=%s\n' "$function"
        printf 'COMMAND=cargo-verus-build-locked-release-no-cheating-output-json-exact-function\n'
    } >"$transcript"
    # Cargo does not fingerprint forwarded Verus selector arguments. Remove only
    # this package's prior artifacts so every row emits its own root query while
    # retaining the already-built dependency closure.
    cargo clean --manifest-path "$repo/Cargo.toml" -p "$package" --release \
        --target-dir "$scratch/verus-target" \
        >"$scratch/$name.clean-transcript" 2>&1 || {
        cat "$scratch/$name.clean-transcript" >&2
        printf 'FAIL: %s could not invalidate the prior selected proof\n' "$name" >&2
        exit 1
    }
    run_child "$transcript" "$repo" env VERUS_Z3_PATH="$verus_root/z3" \
        CARGO_TERM_COLOR=never timeout "$timeout_seconds" \
        "$verus_root/cargo-verus" build -p "$package" --locked --release \
        --target-dir "$scratch/verus-target" --fwd-verus-args-to roots -j 1 -- \
        --no-cheating --output-json --verify-only-module "$module" \
        --verify-function "$function"
    proof_status=$child_status
    [ "$proof_status" -ne 124 ] || {
        printf 'FAIL: %s positive theorem timed out\n' "$name" >&2
        exit 1
    }
    [ "$proof_status" -eq 0 ] || {
        printf 'FAIL: %s positive theorem failed (status %s)\n' \
            "$name" "$proof_status" >&2
        exit 1
    }

    summary="$output/$name.verus.summary"
    python3 -I "$repo/proofs/m1/theorem/check-output-json.py" \
        "$package" "$module" "$function" "$transcript" "$summary" \
        "$actual_version" "$expected_toolchain" >"$scratch/$name.output-check"
    theorem_record="$output/$name.theorem"
    {
        printf 'FORMAT=FERRIC-M1-POSITIVE-THEOREM-V1\n'
        printf 'THEOREM=%s\n' "$name"
        printf 'FOUNDATION=%s\n' "$foundation"
        printf 'OPEN_ASSURANCE_PROPERTY=%s\n' "$property"
        printf 'OPEN_PATH_OBLIGATION=%s\n' "$path_id"
        printf 'VERUS_PACKAGE=%s\n' "$package"
        printf 'VERUS_SOURCE=%s\n' "$source"
        printf 'VERUS_MODULE=%s\n' "$module"
        printf 'VERUS_FUNCTION=%s\n' "$function"
        printf 'COMPILER_PATH=%s\n' "$compiler_path"
        printf 'VERIFIED_MODULES_SHA256=%s\n' "$coverage_digest"
        printf 'SOURCE_SHA256=%s\n' "$source_digest"
        printf 'FUNCTION_SOURCE_IDENTITY_SHA256=%s\n' "$function_source_identity"
        printf 'CARGO_CHECK=passed\n'
        printf 'VERUS_RESULT=proved\n'
    } >"$theorem_record"

    result="$output/$name.result"
    {
        printf 'FORMAT=FERRIC-M1-POSITIVE-RESULT-V1\n'
        printf 'THEOREM=%s\n' "$name"
        printf 'RUN_IDENTITY_SHA256=%s\n' "$(sha256sum "$output/RUN_IDENTITY" | awk '{ print $1 }')"
        printf 'ACTIVE_FOUNDATIONS_SHA256=%s\n' "$(sha256sum "$active" | awk '{ print $1 }')"
        printf 'SELECTED_FOUNDATIONS_SHA256=%s\n' "$(sha256sum "$selected" | awk '{ print $1 }')"
        printf 'VERUS_CLOSURE_TRANSCRIPT_SHA256=%s\n' "$(sha256sum "$closure" | awk '{ print $1 }')"
        printf 'THEOREM_RECORD=%s.theorem\n' "$name"
        printf 'THEOREM_RECORD_SHA256=%s\n' "$(sha256sum "$theorem_record" | awk '{ print $1 }')"
        printf 'THEOREM_RECORD_SIZE=%s\n' "$(stat -c '%s' "$theorem_record")"
        printf 'COMPILE_TRANSCRIPT=ferric-spec.compile.transcript\n'
        printf 'COMPILE_TRANSCRIPT_SHA256=%s\n' "$(sha256sum "$compile_transcript" | awk '{ print $1 }')"
        printf 'COMPILE_TRANSCRIPT_SIZE=%s\n' "$(stat -c '%s' "$compile_transcript")"
        printf 'COMPILE_EXIT_STATUS=%s\n' "$compile_status"
        printf 'VERUS_SUMMARY=%s.verus.summary\n' "$name"
        printf 'VERUS_SUMMARY_SHA256=%s\n' "$(sha256sum "$summary" | awk '{ print $1 }')"
        printf 'VERUS_SUMMARY_SIZE=%s\n' "$(stat -c '%s' "$summary")"
        printf 'VERUS_TRANSCRIPT=%s.verus.transcript\n' "$name"
        printf 'VERUS_TRANSCRIPT_SHA256=%s\n' "$(sha256sum "$transcript" | awk '{ print $1 }')"
        printf 'VERUS_TRANSCRIPT_SIZE=%s\n' "$(stat -c '%s' "$transcript")"
        printf 'VERUS_EXIT_STATUS=%s\n' "$proof_status"
        printf 'RESULT=proved\n'
    } >"$result"
    printf 'PASS: %s compiled and pinned Verus proved %s\n' "$name" "$compiler_path"
done <"$selected"
