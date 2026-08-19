#!/bin/sh
set -eu

usage() {
    printf 'usage: %s ROOT MANIFEST\n' "$0" >&2
    exit 2
}

[ "$#" -eq 2 ] || usage
root=$(CDPATH='' cd -- "$1" 2>/dev/null && pwd) || {
    printf 'FAIL: Verus closure root is unavailable: %s\n' "$1" >&2
    exit 1
}
manifest=$2
[ -f "$manifest" ] || {
    printf 'FAIL: Verus closure manifest is unavailable: %s\n' "$manifest" >&2
    exit 1
}

for tool in find sort stat wc sha256sum awk grep sed tr uname mktemp; do
    command -v "$tool" >/dev/null 2>&1 || {
        printf 'FAIL: %s is required to measure the Verus closure\n' "$tool" >&2
        exit 1
    }
done

unexpected=$(find "$root" -mindepth 1 ! -type d ! -type f -print -quit)
if [ -n "$unexpected" ]; then
    printf 'FAIL: unsupported non-file Verus closure entry: %s\n' "$unexpected" >&2
    exit 1
fi

tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/ferric-verus-closure.XXXXXX")
trap 'rm -rf "$tmp_dir"' EXIT HUP INT TERM
paths="$tmp_dir/paths"
records="$tmp_dir/records"
find "$root" -mindepth 1 -type f -print | LC_ALL=C sort >"$paths"
: >"$records"
while IFS= read -r path; do
    relative=${path#"$root"/}
    case "$relative" in
        ''|*[!A-Za-z0-9._/-]*)
            printf 'FAIL: unsupported Verus closure path: %s\n' "$relative" >&2
            exit 1
            ;;
    esac
    mode_bits=$(stat -c '%a' "$path")
    bytes=$(wc -c <"$path" | tr -d '[:space:]')
    digest=$(sha256sum "$path" | awk '{ print $1 }')
    printf '%s|%s|%s|%s\n' "$relative" "$mode_bits" "$bytes" "$digest" >>"$records"
done <"$paths"

file_count=$(wc -l <"$records" | tr -d '[:space:]')
total_bytes=$(awk -F '|' '{ total += $3 } END { printf "%.0f", total }' "$records")
closure_sha256=$(sha256sum "$records" | awk '{ print $1 }')
vstd_records="$tmp_dir/vstd-records"
grep '^vstd/' "$records" >"$vstd_records" || {
    printf 'FAIL: Verus closure has no vstd source tree\n' >&2
    exit 1
}
vstd_count=$(wc -l <"$vstd_records" | tr -d '[:space:]')
vstd_bytes=$(awk -F '|' '{ total += $3 } END { printf "%.0f", total }' "$vstd_records")
vstd_sha256=$(sha256sum "$vstd_records" | awk '{ print $1 }')

required_record() {
    required_path=$1
    record=$(grep "^${required_path}|" "$records" || true)
    count=$(printf '%s\n' "$record" | sed '/^$/d' | wc -l | tr -d '[:space:]')
    if [ "$count" -ne 1 ]; then
        printf 'FAIL: Verus closure must contain exactly one %s\n' "$required_path" >&2
        exit 1
    fi
    printf '%s\n' "$record"
}

manifest_one() {
    key=$1
    values=$(sed -n "s/^${key}=//p" "$manifest")
    count=$(printf '%s\n' "$values" | sed '/^$/d' | wc -l | tr -d '[:space:]')
    if [ "$count" -ne 1 ]; then
        printf 'FAIL: closure manifest requires exactly one %s field\n' "$key" >&2
        exit 1
    fi
    printf '%s\n' "$values"
}

[ "$(manifest_one format)" = 'FERRIC-VERUS-CLOSURE-V1' ] || {
    printf 'FAIL: unsupported Verus closure manifest format\n' >&2
    exit 1
}
expected_version=$(manifest_one version)
[ -f "$root/version.txt" ] && [ "$(sed -n '1p' "$root/version.txt")" = "$expected_version" ] || {
    printf 'FAIL: Verus closure version.txt does not match the manifest\n' >&2
    exit 1
}
[ "$(manifest_one target)" = 'x86_64-unknown-linux-gnu' ] && \
    [ "$(uname -s)" = Linux ] && [ "$(uname -m)" = x86_64 ] || {
    printf 'FAIL: Verus closure target does not match this host\n' >&2
    exit 1
}

check_required() {
    required_path=$1
    actual=$2
    expected=$(grep "^required=${required_path}|" "$manifest" || true)
    if [ "$expected" != "required=$actual" ]; then
        printf 'FAIL: required Verus closure member drifted: %s\n' "$required_path" >&2
        exit 1
    fi
}
check_required verus "$(required_record verus)"
check_required rust_verify "$(required_record rust_verify)"
check_required z3 "$(required_record z3)"

expected_vstd=$(grep '^subtree=vstd|' "$manifest" || true)
actual_vstd="subtree=vstd|${vstd_count}|${vstd_bytes}|${vstd_sha256}"
[ "$expected_vstd" = "$actual_vstd" ] || {
    printf 'FAIL: Verus vstd source-tree closure drifted\n' >&2
    exit 1
}
[ "$(manifest_one file-count)" = "$file_count" ] || {
    printf 'FAIL: Verus closure file count drifted\n' >&2
    exit 1
}
[ "$(manifest_one total-bytes)" = "$total_bytes" ] || {
    printf 'FAIL: Verus closure total byte count drifted\n' >&2
    exit 1
}
[ "$(manifest_one closure-sha256)" = "$closure_sha256" ] || {
    printf 'FAIL: complete Verus release closure drifted\n' >&2
    exit 1
}

printf 'PASS: pinned Verus release closure matched (%s files, %s bytes)\n' \
    "$file_count" "$total_bytes"
