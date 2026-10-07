# Repeated Matched Starts

`compare_paired_starts.py` aggregates retained HTTP pairs from the existing
V17 or candidate-V2 replay summaries. It does not launch servers. Run on the
approved CPU host, after the frozen per-pair replay accepts the actual raw
cohorts:

```sh
python3 -B compare_paired_starts.py --manifest series.json
```

Example manifest, with paths relative to the manifest file:

```json
{
  "schema": "FerricPairedStartsManifestV1",
  "reference": "reference.json",
  "pairs": [
    {"summary": "start1/pair.json", "ferric": "start1/ferric", "vllm": "start1/vllm"},
    {"summary": "start2/pair.json", "ferric": "start2/ferric", "vllm": "start2/vllm"},
    {"summary": "start3/pair.json", "ferric": "start3/ferric", "vllm": "start3/vllm"}
  ]
}
```

Declare every attempted pair in chronological order, including failed or
missing output. The tool rejects the whole series if any declared attempt
fails; it has no exclusion or best-run option. Launch records must separately
establish roster completeness, fresh starts and a common host clock domain.
The tool cannot discover omitted attempts or independently authenticate
hardware isolation, binaries or the upstream replay.

All starts must use one frozen plan, build, client, reference and vendor image.
Engine order must alternate. The supported workload is TP1/C1, 128 input and
128 output tokens, BF16 decoder with FP32 head, no speculation/prefix caching,
ten warmups and thirty measured requests. Other cells need separate contracts.

Output contains each pair's TTFT, TPOT and measured-span output-rate ratio,
plus descriptive spread across starts. One or two pairs are limited screening;
three or more valid alternating pairs are repeated screening. Neither is
performance qualification. There are no confidence intervals, pooled tail
estimates or sustained-goodput claims. `competitiveness_accepted`,
`qualification` and `framework_win_claim` remain false.

TTFT ends at the first nonempty SSE text chunk. TPOT divides the first-to-last
text span by 127, not individual token interarrival. Output rate includes
inter-window gaps and drain. These definitions match the retained input format.
