# Resident-State Intake

Input schema: `ferric-p228-resident-state-decode-observation-inputs-v1`.
Package schema: `ferric-p228-resident-state-decode-observation-package-v1`.
Engineering review: `ferric-p228-resident-state-decode-engineering-review-v1`.
Pure receipt: `ferric-p228-resident-state-decode-pure-v1`.

The old input fields are preserved, with one mandatory `prior_deployment`
FilePin added. It must equal the new deployment manifest's CPU475 prior pin.
The engineering review binds this pin as well as every existing request,
runtime review, image deployment and six-case prerequisite. Its
`historical_runtime` remains the authenticated CPU475 parent/worker/image map;
its selected parent/worker/image fields identify CPU633/CPU522/V7 respectively.
No review is constructed by this package.

Fresh GPU labels are
`prefix-resident-state-decode-{tf4|ar4}-{policy}-gpu-v228-vN`, with the same
three closed host policies as before. Fresh pure labels are
`resident-state-decode-pure-v228-vN`. Old schemas/labels do not authorize this
successor. Existing runtime review schema and substantive library checks are
unchanged; an old worker review fails because its exact binary pin differs.

`deployment(pins, plan, old)` calls the new verifier with the frozen CPU633
historical-deployment callback, then calls `old.deployment` for the explicit
CPU475 prior. It returns `(new_manifest, new_runtime, prior_manifest,
historical_runtime)`. `runtime_generations` requires identical CPU633 base,
parent and historical image plus the exact newly qualified worker. Source,
raw CPU522 test replay, ELF and alias closure remain verifier responsibilities.

Context preserves every old API/key and adds parsed `prior_deployment` and
`prior_deployment_pin`. `selected_runtime` and `runtime` use CPU522 plus V7;
`base_deployment` still records CPU633, while `deployment` is CPU522's manifest.
The frozen comparator, source/ISA/arithmetic assumptions, six GPU case replays,
six numerical receipts, request checks, runtime library checks and both input
ledger rechecks remain unchanged. No numerical reference is rerun during GPU
cleanup and no old paired-image equality gate is restored.

Final package roster/count placeholders are deliberately fail-closed until root
stages all exact members. `TEST_RUNNER_SHA` identifies the authored root runner,
not a successful execution. Actual pure tests, exporter/receiver replay, runtime
audits and substantive engineering review are independent required inputs.
