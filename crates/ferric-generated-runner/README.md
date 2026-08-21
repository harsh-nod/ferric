# Ferric generated Qwen3 gfx942 runner declarations

This crate is the checked-in output of Ferric's deterministic M1 runner
declaration generator. It contains the exact target-then-draft B3 plan order,
operation offsets and counts, and a request-independent schema for logical
scalar inputs. The workspace compiles this file, and `ferric-build` tests that
regeneration produces byte-for-byte identical source.

The declarations are inert. They contain no addresses, allocations, device
objects, queues, packets, artifacts, loaders, launch operations, completion
handling, graph-refinement proof, hardware observation, performance result,
or qualification evidence. They do not close an M1 roadmap item.
