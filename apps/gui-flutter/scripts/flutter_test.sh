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
#  2. Forward moon's passthrough args. `moon run gui-flutter:test -- --plain-name foo`
#     appends them after the task args. When the command is `bash -c <script>` bash
#     maps them onto $0/$1..., so an arg-shaped $0 is folded back in; when the script
#     runs by path, $0 is that path and must NOT be forwarded, or `flutter test` tries
#     to load it as a suite ("Missing definition of main").
#  3. nproc is affinity-aware (4 under `taskset -c 0-3`, unlike getconf, which reports
#     the full CPU count), so it stays first. getconf _NPROCESSORS_ONLN is the POSIX
#     fallback for macOS/BSD, where nproc does not exist (there `$(nproc)` expands
#     empty and `flutter test -j ""` fails). 4 is the last resort.
set -euo pipefail

unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_PREFIX

case "$0" in
  -*) set -- "$0" "$@" ;;
esac

jobs=$(nproc 2>/dev/null || getconf _NPROCESSORS_ONLN 2>/dev/null || echo 4)

exec mise exec -- flutter test -j "$jobs" "$@"
