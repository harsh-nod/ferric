# Retained Native Gate/Up Experiment

The gate/up a009 CPU-qualified source is preserved independently from the
down-only worker integrated in the engineering adapter. `manifest.json` lists
the exact 1,441-file source closure; `source-overlay/` contains only bytes that
differ from the publication tree. This includes historical documentation and
fixtures to retain the original source identity, not to replace current docs.

On the remote build host, reconstruct into a new directory outside the checkout:

```sh
python3 -I -B experiments/native-gate-up-a009/restore.py \
  --repository "$PWD" --output /path/to/new/private/gate-up-source
```

Restoration checks every input hash before writing and refuses source drift.
It copies only the original closed file set, so down-only files and unrelated
new modules are absent. It does not build, run a GPU, grant artifact admission,
or combine gate/up with down-only. Build the restored tree with its existing
pinned dependencies and the bounded remote profile.

The subsequent a004 native campaign was numerically correct but inconclusive:
only four of six adjacent timing pairs improved. Historical CPU qualification
and source preservation are not performance qualification or default promotion.
