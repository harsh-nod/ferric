# Failed Build Cache Retirement

This completed maintenance operation removed only `target` and empty `tmp`
from the failed diagnostic CPU V1 generation on `mi350`. Its 2,028 cache files
had 2,445,502,744 logical bytes. The source tree, failed receipt, all 79 raw
files, controller, input manifest and archived evidence remained untouched;
83 retained pins were checked again after deletion. No successful build's
products were removed.

An earlier unprivileged attempt stopped before deletion because protected
process metadata was unreadable. This attempt used root only for inspection,
then reset supplementary groups, GID and UID to the owning user before
deleting the two exact task-owned cache directories. The same process-reference
checks remained mandatory. They establish a point-in-time observation, not a
lock against future cache users.

Filesystem free space increased from 43,569,565,696 to 45,797,580,800 bytes
across the operation. These are shared-filesystem observations, not a disk
performance measurement.

The [receipt](receipt.json) is an unchanged copy of the remote record:
`38804540d0aad58a0c7663b4d93a373e2356b61d2efd2af76c09b9f59f172db5`.
The reviewed [controller](controller.py) is
`c8de82cc8252f9253085925cd35ae4b93584e2cac704dd4f2d61ed45d6250f02`.
This operation does not qualify a compiler, kernel or model.
