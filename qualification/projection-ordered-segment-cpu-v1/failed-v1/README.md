# Preserved First CPU Attempt

This attempt failed its final `parent-default-check` phase with exit code 101.
Two new exports were missing the `tp-batch-engineering` feature guard; nine
unresolved-import/module diagnostics are retained in the check output. The
successful earlier tests and five executable builds do not qualify this run.

`failed.json` is copied verbatim from the original remote attempt. Its SHA256 is
`dd363e5d49249d5096418a97c574a2cf4852b342658e922eea922049f94040c4`.
The command, result, stdout and stderr bodies are also copied verbatim and
rehash to the corresponding `raw` entries in that receipt. The source tree,
earlier raw phases and started record remain retained outside this publication;
this directory does not purport to contain the entire failed attempt.

These five supplemental files and this explanation were added by the primary
agent after the successful-run publisher completed. They are not entries in
the successful publication's `result.json` copy ledger. The failed run is not
relabeled; the separate corrected V2 run passed all 75 phases.
