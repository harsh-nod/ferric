#!/usr/bin/env bash
set -euo pipefail
umask 077
root=${1:?owned remote stage required}
mode=${2:?check mode required}
export PATH=/home/harsh/.cargo/bin:$PATH
export CARGO_BUILD_JOBS=1
export CARGO_TERM_COLOR=never
export RUSTC_BOOTSTRAP=fe2o3_device,fe2o3_macros
cd "$root/source"
case "$mode" in
    tests)
        timeout 1200 cargo test --manifest-path adapters/m1-engineering-execution-v1/Cargo.toml --locked --lib --release tp_execution --target-dir "$root/target" -j 1 >"$root/driver-tests.log" 2>&1
        timeout 1200 cargo test -p ferric-engine --locked --lib --release tensor_parallel_execution --target-dir "$root/target" -j 1 >"$root/sequence-tests.log" 2>&1
        ;;
    clippy)
        timeout 1200 cargo clippy --manifest-path adapters/m1-engineering-execution-v1/Cargo.toml --locked --lib --tests --release --target-dir "$root/target" -j 1 -- -D warnings >"$root/clippy.log" 2>&1
        ;;
    proof)
        verus=/home/harsh/.cache/fe2o3-verus-0.2026.08.02.b677dd5
        export VERUS_Z3_PATH="$verus/z3"
        timeout 1800 "$verus/cargo-verus" build -p ferric-engine --locked --release --target-dir "$root/verus-target" --fwd-verus-args-to roots -j 1 --lib -- --no-cheating --output-json --verify-only-module tensor_parallel_execution >"$root/proof.log" 2>&1
        ;;
    identity)
        python3 proofs/check-source.py --verus-blocks crates/ferric-engine/src/tensor_parallel_execution.rs >"$root/lexical.log" 2>&1
        bash proofs/verify-verus-closure.sh /home/harsh/.cache/fe2o3-verus-0.2026.08.02.b677dd5 proofs/verus/VERUS_CLOSURE_MANIFEST >"$root/verus-closure.log"
        python3 proofs/source-closure.py "$root/source" "$root/source-closure.txt" >"$root/source-closure.log"
        rustc --version --verbose >"$root/rustc-version.log"
        ;;
    *) exit 2 ;;
esac
