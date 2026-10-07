# Bank Rearm CPU Attempt 1

This original MI350 qualification attempt failed during the worker test build.
It is not a successful coupled qualification, GPU run or performance result.
No proposed runtime or worker source was integrated from this attempt.

The complete runtime suite passed 1,152 tests with eight existing ignored
tests. All nine focused scopes passed, including 22 mixed-bank tests. Ten
selected facade doctests and eight parser regression checks also passed.
The worker library test build then exited 101: the new empty-input CLI test
used an unsafe block without the function-local allowance required by the
crate's `-D unsafe-code` lint. The compiler diagnostic is preserved in the
original Cargo JSON output, not only the stderr summary.

All 22 phases exited naturally and were reaped, with their process groups
absent. The first 21 exited zero. Source, cache, dependency and executable
postchecks passed. Elapsed qualification time was 111.173 seconds. No worker
test outcome, final worker executable or native execution is inferred.

The capsule retains 169 originals, including 117 raw bodies, ten lineage
inputs, all 23 actual formatted proposal postimages and the original failed
terminal. Six runtime test executables are authenticated by metadata; their
binary bodies are not included. Storage and execution limits were unchanged.

- Terminal: 2,145,330 bytes, `701afc4243aa5187190b3238c9debdea02bc87cd9969d287d106478542f9c741`.
- Final source map: 423,235 bytes, `ebfe3a960ec66178d84511635d80936b3c2914f728db58cc8a322e602fab9b7f`.
- Archive: 1,704,499 bytes, `2530a4725f59d14308aee06487e39b75cec93efe316b5dff1ec80c8ff7f17bd7`.

The original failure remains unchanged. A repaired test requires a separate,
fresh complete runtime and worker qualification before integration.
