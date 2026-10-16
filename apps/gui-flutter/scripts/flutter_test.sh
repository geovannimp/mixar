#!/usr/bin/env bash
# Runs the Flutter test suite with the concurrency the shared `test` task wants.
#
# Why a script instead of inline `bash -c` args in moon.yml: moon's task-args parser
# rewrites the string (it strips `-- <...>` and `$...` sequences), so a multi-rule
# one-liner cannot survive the trip. This mirrors `scripts/ensure-frb-codegen.sh`,
# which the `generate` task already invokes by path.
#
# Three rules, all learned from review of the inline version:
#
#  1. Unset GIT_* first. Flutter's version probe inherits the hook's repo and reports
#     0.0.0-unknown, which is why `format-files`/`lint-files`/`icons` do the same.
#  2. Forward moon's passthrough args. moon runs this as
#     `bash scripts/flutter_test.sh <args>`, so they arrive in $1.. and "$@"
#     forwards them (verified: a bogus flag fails loudly, a real filter selects
#     one test). There is deliberately no `case "$0" in -*)` fold here: moon
#     cannot invoke this task as `bash -c '<script>'` anyway, because it mangles
#     inline script strings (it strips `-- <...>` and `$...` sequences).
#  3. nproc is affinity-aware (4 under `taskset -c 0-3`, unlike getconf, which reports
#     the full CPU count), so it stays first. getconf _NPROCESSORS_ONLN is the POSIX
#     fallback for macOS/BSD, where nproc does not exist (there `$(nproc)` expands
#     empty and `flutter test -j ""` fails). 4 is the last resort.
set -euo pipefail

unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_PREFIX

jobs=$(nproc 2>/dev/null || getconf _NPROCESSORS_ONLN 2>/dev/null || echo 4)
# `nproc` can print 0 when every CPU is offline or the cgroup hides them, and
# `flutter test -j 0` is an invalid argument, so fall back rather than forward it.
case "$jobs" in
  '' | *[!0-9]*) jobs=4 ;;
esac
[ "$jobs" -gt 0 ] || jobs=4

exec mise exec -- flutter test -j "$jobs" "$@"
