# Source-gate dependency closures

The source gate keeps three dependency policies separate:

- `../../RUNTIME_DEPENDENCY_TCB` is the established workspace runtime crates.io
  closure.
- `VERIFIER_PRODUCTION_DEPENDENCY_TCB` starts at the protected verifier in the
  root workspace resolution and admits only its normal and build edges.
- `VERIFIER_DEV_DEPENDENCY_TCB` starts at the standalone verifier workspace and
  admits its complete normal, build, and dev resolution generated with
  `cargo metadata --locked --all-features`.

The verifier TCBs bind normalized package identities, registry checksums from
the applicable lockfile, resolved features, complete Cargo target metadata,
manifest declarations, and edge kind/target topology. Local package paths are
accepted only for the exact verifier, device, and source-pin directories. Git
packages are accepted only from the exact reviewed fe2o3 and pliron revisions.
The isolated metadata package graph must equal the standalone verifier lock
closure, so a dev package cannot be hidden by the root workspace resolution.

These files are co-located review records, not an external signature or a claim
that third-party code is correct. Updating a lockfile, manifest, feature set,
target, or dependency edge requires regeneration and review of the resulting
TCB diff. `proofs/qualify-release.sh` regenerates and compares both records and
runs the source-gate unit suite in locked release mode before using the gate.

Registry and Git `manifest_path` and target paths are observations, not source
authentication. The qualifier creates both Cargo 1.97 metadata documents itself
from the locked, read-only source snapshot, stores them below its private
`mktemp` directory, and passes those fresh documents directly to the source
gate. Registry checksums and exact Git revisions remain the source authorities.
The gate requires every observed target to remain below its observed package
manifest directory and requires first-party manifests to be the exact reviewed
repository paths; it does not claim to authenticate an arbitrary caller-supplied
registry checkout independently of this qualification boundary.
