# Graph-Work Failure Diagnostics

The [qualified DAG-cache compiler](../guarded-mlp-indexed-atomic-dag-v1/README.md)
still refuses the actual guarded gfx950 kernel at its graph-analysis work limit.
The previous error did not identify the function or charging operation. This
isolated diagnostic build adds that context without changing admission.

## What Changes

The central charge routine retains its checked addition and 3,145,728-unit
ceiling. Equality succeeds. Overflow leaves the counter unchanged; a finite
over-limit sum is assigned before refusal. The atomic forwarding routine still
commits its separate counter only after success.

Only a failure allocates a diagnostic record. It contains the prior counter,
requested amount, checked sum, limit and static caller location. Propagation
adds the actual semantic function identity and source, and the selected root
and body when available. The first context wins. Missing body information does
not get replaced by a potentially misleading logical-root declaration.

Caller paths retain at most 128 raw bytes, rendered as at most 512 escaped
ASCII bytes. Export and logical names are also bounded. No graph traversal,
backtrace, environment override, new cache or success-path allocation is added.
All atomic, alias, source-origin and graph-admission predicates stay unchanged.
The earlier capacity expansion and DAG cache remain inherited baseline changes,
not changes made by this diagnostic patch.

## Qualification

The first qualification attempt on `mi350` failed one compiler test. The
compiler library reported 1,265 passed, one failed and 24 ignored; Pliron
passed 1,507 tests with one ignore. All fifteen executed phases exited
naturally and were reaped, with unchanged sources and clean postchecks.
The compiler test process returned 101. Elapsed time was 230.162137 seconds.

The failing `induction_body_operand_rejects_header_latch_and_outside_uses`
assertion incorrectly expected the new graph-work error. This path first calls
the separate alias resolver, whose checked addition rejects an exhausted
counter with the existing `Unsupported` alias-overflow error. The correction
matches that exact message and verifies that the counter remains `usize::MAX`.
It changes only the fixture; the diagnostic implementation is unchanged.

The retry retains the complete compiler and Pliron suites, all earlier focused
and extraction controls, and nine diagnostic tests plus their focused repeat.

The new tests exercise exact and exceeded limits, overflow, distinct call
sites, both real forwarding methods, atomic-counter noncommit on failure,
actual body attribution, missing body context, first-context preservation,
bounded escaping and unchanged unrelated errors. Existing negative tests keep
their inputs and refusal conditions while recognizing the new typed error.

Successful completion would mean 37 phases, 24 scopes, 2,858 passing test
executions and 25 historical ignores. Repeated executions are not unique tests.
CPU qualification alone cannot identify the guarded kernel's actual failing
charge site. A fresh loader inspection and guarded gfx950 compile remain
required, followed separately by GPU and independent numerical validation.

This is not production admission, a GPU result or a decode performance claim.
All issue #42 milestones and the 700 tokens/s target remain open.

## Failed Attempt Evidence

[attempt-v1](attempt-v1) retains the failed receipt, executed harness, source
overlay, baseline lineage and all 79 raw records. The archive contains 110
members and 109 pinned bodies, totaling 33,601,291 body bytes. Receipt SHA-256:
`cc75239a5b500eaf515ebe904e0710c2852864efabdaf466d7de4b10a82870eb`.
The 5,513,950-byte archive SHA-256 is
`6f3782741c51645ce2aba2e1fa79749bb6af324a58e51cbb6a72e399f9b30740`.
No loader inspection or guarded compilation used this failed generation.
