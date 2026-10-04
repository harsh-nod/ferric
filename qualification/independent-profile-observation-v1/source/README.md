# Independent Profile Observation Adapter Draft

This is an unfrozen, no-launch Python adapter. Its new code and tests have not
been imported, syntax-checked, or executed by the author. No SSH, Cargo, GPU,
receipt writing, artifact deployment, or publication was performed. Root owns
review, freezing, pure tests, and any later integration or execution.

## Implemented Scope

`observe.py` is the narrow replacement for the *parent report validation and
numerical consumer* in the frozen `p227-prefix-parity-observation-v4/run_case.py`.
It is not a replacement controller and deliberately has no launch CLI. The old
`prepare.load`, deployment reader, and historical SourceV5 provenance are not
called or relabelled. No future native binary or candidate HSACO hash is guessed.

`validation.py` and `child_evidence.py` are byte-identical frozen V4 copies:

- Validation SHA256: `cdaf6dc53208bbca8f23b2a3fa3eca9d28fe00ca4cfc74b36d225f3ebe8d28d1`.
- Child custody SHA256: `2cbb74ada0950d1767c8b526008e9d9c7efb48d84649e1aea8f3d6661309edd3`.

The adapter reuses `validation.base`, `request`, `review`, `terminal`, `finite`,
and strict parsing/pins. It never invokes the old `inspection`/`observation`
wrappers that require paired equality. The child custody `validate` and
`records` functions are unchanged, including all ten sidecars, natural waits,
actual parent/descendant identities, pinned executable-FD child argv, inherited
owned group, exact request/input/capture joins, and both native Close reports.

The new closed inspection/observation schemas match the frozen Rust adapter
`p228-independent-native-profile-v1`. Legacy parity reports, failure reports,
extra fields, paired comparison rows, or a claimed bitwise match are rejected.
Both profiles must have valid typed terminal states and complete finite computed
buffers/current KV slots. Complete KV buffers are hashed, including untouched
history/future bytes, but this adapter does not reconstruct their initial values.
Native `validate_run` owns that check; the independent comparator owns causal
finite checks and current append math. No historical-KV numerical-origin or
untouched-byte revalidation is claimed here.

## No-Launch API

The integration entry is:

```text
compare_retained(plan, P, pins, read, profile_root, stage_root, sidecar_root)
```

`P` and `pins` are the existing frozen custody API/ledger from the versioned
controller. In particular `P.read`, `P.document`, `P.ENV`, `P.R`, `P.E`,
`pins.records`, and `pins.recheck` retain their original semantics. `read` is the
unchanged numerical sidecar `Reader`, with its own exact FilePin validation and
`recheck`. These are explicit integration dependencies, not new compatibility
fallbacks or alternate provenance loaders. The adapter returns data in memory;
it does not author a successful run receipt.

The closed plan schema is
`ferric-p228-independent-profile-observation-inputs-v1`, with exactly:

- `case`: one of the unchanged six native cases.
- `case_directory`: `E/prefix-independent-profile-gpu-v228-vN/CASE`.
- `request`: actual pinned native V6 request with its six scoped reviews.
- `binary`: actual pinned new native executable, independently qualified later.
- `inspection_result` and `native_result`: actual owned leaf result pins at
  `CASE/inspection/result.json` and `CASE/native/result.json`.
- `output_weights`: the two ordered original O-weight FilePins.

Every FilePin retains `path`, `bytes`, and `sha256`. No example plan containing
invented successful receipt, binary, or HSACO pins is supplied. The request
determines the real baseline/candidate objects and exact review scope. Their
deployment/source/ISA qualification must be supplied separately by root.

The API replays both exact owned leaf commands with the new explicit selectors:

```text
--inspect-independent-profiles-v1 REQUEST SHA256
--execute-reviewed-engineering-prefix-independent-profiles-v1 REQUEST SHA256
```

It keeps the original 180-second leaf deadline, CPU affinity/niceness, memory and
stream bounds, environment, natural exit, no cleanup signals, absent groups,
reaped processes, command/start joins and empty stderr. These are retained-result
checks, not execution of those commands. It then authenticates the four full
capture files and unchanged child sidecars. All custody pins and Reader inputs
are rechecked before numerical work and after each profile and final census.

## Independent Numerical Work

Only after authenticated captures and custody rechecks does the adapter load
the unchanged `p228-prefix-profile-numerical-v1/compare_profile.py`, SHA256
`4b72aeac6167006151b258536cc834f39e7a55da15ec10744df2af438bf3b46d`.
Its existing frozen stage, attention, O-projection references and bounds are
unchanged, including the exact NumPy 2.2.6 requirement. The caller supplies the
three package roots; each existing loader authenticates its own fixed helpers.

The adapter constructs one exact numerical plan per authenticated profile,
binding that profile's two capture pins, original input hashes, baseline request,
and rank-local O weights. No baseline/candidate output is substituted for the
other. Result rank, capture, per-stage hashes, weights, case, profile, and inert
authority flags are checked again. Numerical reads also join the custody ledger.
The unchanged Reader registers a pin only after successful authentication, so a
narrow `CustodyReader` records each requested pin first, authenticates it through
`P.read`, and verifies the underlying Reader's returned bytes. Any first-read,
pin-conflict, or byte-authentication failure raises `NumericalCustodyError`, which
is rethrown before the numerical-rejection handler and prevents the next profile.

A numerical rejection is retained separately for that profile and does not
accept or skip the other profile. A custody recheck failure aborts the entire
adapter and cannot be downgraded to a numerical failure. Returned
`conditional_operator_checks_passed` is true only if both independent calls
returned correctly joined conditional results. Per-profile outcomes remain
visible. Neither result grants independent numerical acceptance or production
authority; actual new-source arithmetic prerequisites are still external gates.

## Required Controller Integration

No end-to-end runtime is provided by this draft. A future versioned controller
must retain the frozen `run_case.py` ownership structure rather than bypass it:

1. Supply actual CPU-qualified new native binary, deployment, dynamic ELF/runtime
   audit and new candidate-image/source/compiler/finalizer/ISA provenance. Old
   SourceV5 lineage cannot be reused for the new candidate.
2. Supply fresh six scoped reviews for the exact case/image/device geometry and
   independently review the numerical arithmetic prerequisites against the new
   emitted ISA. Authenticating old numerical policy metadata is not that review.
3. Preserve `bounded`, `leaf`, `audit`, `resources`, `inventory`, `quiescent`,
   signal restoration, outer-owned cleanup/reap, one native attempt/no retries,
   reserved post-audit deadlines, all three pre- and three post-device audits,
   exact boot/topology/process checks, and final `P.guard`/pin rechecks. Post-audits
   must still execute after numerical or source/custody failure.
4. Replace the two native selectors and only the old parent inspection/observation
   consumers in `execute` and prior-case replay. Keep `C.validate` and the ten
   sidecars. Prior-case receipts require a new closed schema and must recompute
   independent results; old parity `complete.json` is never sufficient.
5. Run numerical comparison in a separately bounded CPU ownership scope or after
   the native controller has completed mandatory post-audits. Do not insert an
   unbounded reference calculation into the reserved GPU cleanup/audit window.
6. Join actual adapter pure-test qualification and retained numerical results
   before publishing any runtime outcome. Recheck both ledgers and all original
   package/fixture/source inputs at the publication boundary.

The adapter explicitly reports false for deployment/platform-audit verification,
GPU execution verification, arithmetic-prerequisite verification, independent
numerical/full-prefix/full-model acceptance, production authority, performance,
scheduler guarantees, historical KV numerical-origin, and untouched-KV byte
revalidation. It makes no 700-token/s or end-to-end model claim.

## Authored Tests

`test_observe.py` contains synthetic closed-schema, complete-capture, numerical
plan/result identity, owned leaf envelope, and reference-routing tests. Its
positive math results are explicit mocks, not real-reference or GPU evidence.
It checks finite differences without parity, legacy/failure schema refusal,
input/image/review drift, both profile state/Close checks, exact extents and
hashes, nonfinite computed stages, position-16 KV slot selection, false authority
claims, separate baseline/candidate rejection, and fatal recheck failures.
First-read custody failure before the underlying Reader registers a pin, failure
before the underlying Reader is called, conflicting requested pins, and wrong
returned bytes have explicit refusal tests.
The frozen helper bytes are also checked. None of these tests has been run by
the author. Existing real-reference comparator tests and actual captured data
remain separate required qualifications.
