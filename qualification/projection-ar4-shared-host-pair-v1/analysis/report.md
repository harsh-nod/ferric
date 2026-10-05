# Single-Pair Host Observations

Default first; shared second. Warm-cache/order confounding is unresolved.
Counter intervals include surrounding work, so their forward-wall ratios are not a disjoint latency partition.
No GPU-time, independent numerical, sustained throughput, or production claim.

| Route | Interval | Host s | Forward s | Full-currentness s | Full / interval |
|---|---|---:|---:|---:|---:|
| default | fresh_enabled -> setup_sealed | 167.073904 | - | 128.605162 | 0.769750 |
| default | setup_sealed -> forward_0 | 10.556722 | 10.556575 | 9.853116 | 0.933350 |
| default | forward_0 -> forward_1 | 10.571357 | 10.564497 | 9.849011 | 0.931670 |
| default | forward_1 -> forward_2 | 12.492906 | 12.486571 | 11.777102 | 0.942703 |
| default | forward_2 -> forward_3 | 12.483778 | 12.477322 | 11.769090 | 0.942751 |
| default | forward_3 -> before_close | 0.006514 | - | 0.000000 | 0.000000 |
| shared | fresh_enabled -> setup_sealed | 89.953356 | - | 51.543174 | 0.572999 |
| shared | setup_sealed -> forward_0 | 5.245222 | 5.245102 | 4.536995 | 0.864977 |
| shared | forward_0 -> forward_1 | 5.269665 | 5.263024 | 4.562654 | 0.865834 |
| shared | forward_1 -> forward_2 | 5.762170 | 5.755813 | 5.048430 | 0.876134 |
| shared | forward_2 -> forward_3 | 5.774303 | 5.767707 | 5.063610 | 0.876921 |
| shared | forward_3 -> before_close | 0.006464 | - | 0.000000 | 0.000000 |

Separate timing (nanoseconds; configuration is outside zero-based snapshots):
```json
{
  "default": {
    "setup_interval_host_ns": 167073904487,
    "forward_host_ns": [
      10556575434,
      10564496619,
      12486571230,
      12477321865
    ],
    "serialization_host_ns": [
      2606591,
      4764643,
      2518282,
      2491601
    ],
    "close_host_ns": 26449168395,
    "configuration_host_ns": null,
    "final_serialization_interval_host_ns": 6514104
  },
  "shared": {
    "setup_interval_host_ns": 89953355941,
    "forward_host_ns": [
      5245101512,
      5263023592,
      5755813128,
      5767707055
    ],
    "serialization_host_ns": [
      2609211,
      2492572,
      2497202,
      2476962
    ],
    "close_host_ns": 12514136128,
    "configuration_host_ns": 22003624,
    "final_serialization_interval_host_ns": 6463864
  }
}
```

Payload repeatability: [true, true, true, true]
