# Position-5 Reference Proposal

Unexecuted derivative of the qualified Readiness40 reference source manifest
`97dcb5a8dd5e6c986cd8fa792670ab184d77dfeaeb2c742d4a837d07723297ca`.
Only the four captured positions change from `[0,15,16,39]` to `[0,5,16,39]`;
the source, owner and result namespaces are distinct. The original remains immutable.

Two fresh independent DynamicCache passes each execute all 40 authentic prompt
positions, using the same full 2048-token prompt, checkpoint, deterministic BF16
policy, FP32 RoPE, math SDPA, image and private package overlay. No native data
is consumed in reference generation. No generated token replaces a prompt input.
The source retains 20 pure CPU tests before framework launch; they have not been
executed for this proposal. Five loader/ownership/diagnostic helper bodies are
byte-identical to the qualified reference. Resource, ownership, retirement,
idle, source/model/package postcheck and 1200+600 second bounds are unchanged.

Remote fresh root:
`E/guarded-mlp-readiness40-position5-reference-v228-v1`, with source/, inputs/,
and a root-pinned fresh launch-plan.json using schema
`ferric-readiness40-position5-reference-launch-v1`. Root owns staging and invocation
of `python3 -I -B source/launch.py PLAN_SHA`; no future plan or outcome is invented.

The independent comparison must first validate both repeats, all 40 original
input/logit pins, and common full captures at positions 0,16,39. New position-5
metrics do not establish numerical acceptance or a performance claim. The known
original native/reference argmax disagreement at position 5 remains unresolved.
