# Ordered Segment Host Counter Scope

Read-only source interpretation, not a measurement. Runtime proposal manifest
0157f78f preserves per-kernel calls to Context.prepare_dispatch_with_peer_bindings
(engineering_gfx950.rs:782), hence dispatch_prepare_ns and kernel_admissions.
Context.profile_started is enabled by host observation even though legacy
performance profiling is off. Ordered publish_fixed records dispatch_publish_ns;
ordered complete records dispatch_wait_ns and adds two dispatches per rank.
Packet counts therefore still count kernels, not one count per compound segment.

The publish/wait scopes are not equivalent to the old single-packet path:

* Ordered stage copies kernargs and resets both signal slots before the publish
  timer. The old single-packet publish timer included those operations.
* Ordered wait spans each batch doorbell through both ranks' completion and all
  four signal validations, until that rank's delayed retirement. Rank intervals
  overlap and include peer publication/poll work. They must not be summed as a
  nonoverlapping makespan or compared as individual residual/MLP durations.
* completion_polls counts final-slot polling only; checking the earlier signal
  is a separate validation. A smaller count does not establish less GPU work.
* Typed initial/terminal MLP atomic state reads are outside generic read counters
  in both paths. New kernarg staging is outside prepare/publish/wait timers,
  although its nested full-currentness checks remain in currentness counters.

Every actual Context full-currentness call still records rank-full time, and
the Group/publication full checks still record their shared counters. Rank-full
plus shared-group plus shared-publication time is a defined nonnested total of
those currentness calls; it overlaps prepare/wait and is not added to them.
Different required fences/poll placements change the work counted. This is not
a policy-neutral rewrite or a counter-derived speedup claim.

segment_host_ns is the new inclusive preflight-to-retirement interval, including
first-use arena allocation, staging, all fences and state reads. Compare matched
same-generation enclosing forward wall times and outputs as the primary native
observation. Disclose cold first use, scheduling, run order, setup/readback and
single-trial limitations. Do not compare inclusive segment time directly with
the old dispatch_elapsed_ns pairs, or invent residual/MLP sub-timings.
