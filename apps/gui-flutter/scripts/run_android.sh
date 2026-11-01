#!/usr/bin/env bash
# `flutter run` takes a concrete device id, not a platform: `-d android` fails
# with "No supported devices found". Resolve the first attached Android
# device/emulator and run there, forwarding moon's passthrough args.
#
# GIT_* is unset for the same reason as scripts/flutter_test.sh: the version
# probe otherwise inherits a hook's repo and reports 0.0.0-unknown.
set -euo pipefail

unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_PREFIX

device=$(
  mise exec -- flutter devices --machine |
    python3 -c '
import json, sys

devices = [
    d for d in json.load(sys.stdin) if d.get("targetPlatform", "").startswith("android")
]
if not devices:
    sys.exit(
        "No Android device or emulator attached "
        "(start one with `flutter emulators --launch <id>`)"
    )
print(devices[0]["id"])
'
)

exec mise exec -- flutter run -d "$device" "$@"
