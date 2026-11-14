#!/usr/bin/env python3
"""Exercise macOS slice orchestration without Xcode or a network connection."""

import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
SCRIPTS = ROOT / "apps/gui-flutter/rust_builder/cargokit"

# Fake native tools model architectures as file contents. The real macOS release
# job additionally checks the built Mach-O slices and executable load commands.
TOOLS = r'''#!/usr/bin/env python3
import json, os, pathlib, shutil, sys
name = pathlib.Path(sys.argv[0]).name
args = sys.argv[1:]
with open(os.environ['TOOL_LOG'], 'a') as log:
    log.write(json.dumps([name, args]) + '\n')
if name == 'curl':
    shutil.copyfile(os.environ['ORT_ARCHIVE'], args[args.index('--output') + 1])
elif name == 'shasum':
    sys.exit(int(os.environ.get('BAD_CHECKSUM', '0')))
elif name == 'install_name_tool':
    assert args[:2] == ['-id', '@rpath/libonnxruntime.dylib']
elif name == 'lipo':
    if args[0] == '-create':
        output = args.index('-output')
        contents = ' '.join(pathlib.Path(p).read_text() for p in args[1:output])
        pathlib.Path(args[output + 1]).write_text(contents)
    elif args[0] == '-verify_arch':
        architectures = pathlib.Path(args[-1]).read_text().split()
        sys.exit(0 if all(a in architectures for a in args[1:-1]) else 1)
    else:
        sys.exit(1)
'''

BUILD_TOOL = '''#!/bin/sh
set -eu
printf '%s|%s|%s\\n' "$CARGOKIT_DARWIN_ARCHS" "${ORT_LIB_LOCATION:-}" "${ORT_PREFER_DYNAMIC_LINK:-}" >> "$BUILD_LOG"
if [ "${FAIL_ARCH:-}" = "$CARGOKIT_DARWIN_ARCHS" ]; then exit 1; fi
printf '%s' "$CARGOKIT_DARWIN_ARCHS" > "$CARGOKIT_OUTPUT_DIR/libhost_flutter.a"
if [ "$CARGOKIT_DARWIN_ARCHS" = arm64 ]; then
  printf arm64 > "$CARGOKIT_OUTPUT_DIR/libwebgpu_dawn.dylib"
fi
'''


class MacOSBuildTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.scripts = self.root / "scripts with spaces"
        self.scripts.mkdir()
        for name in ["build_macos_slices.sh", "prepare_macos_ort.sh"]:
            shutil.copyfile(SCRIPTS / name, self.scripts / name)
        (self.scripts / "run_build_tool.sh").write_text(BUILD_TOOL)
        tools = self.root / "tools"
        tools.mkdir()
        for name in ["curl", "shasum", "install_name_tool", "lipo"]:
            tool = tools / name
            tool.write_text(TOOLS)
            tool.chmod(0o755)
        archive = self.root / "runtime.tgz"
        with tarfile.open(archive, "w:gz") as tar:
            member = tarfile.TarInfo(
                "onnxruntime-osx-x86_64-1.23.2/lib/libonnxruntime.dylib"
            )
            member.size = len(b"x86_64")
            tar.addfile(member, io.BytesIO(b"x86_64"))
        self.output = self.root / "output with spaces"
        self.env = {
            **os.environ,
            "PATH": str(tools) + os.pathsep + os.environ["PATH"],
            "TOOL_LOG": str(self.root / "tools.log"),
            "BUILD_LOG": str(self.root / "build.log"),
            "ORT_ARCHIVE": str(archive),
            "CARGOKIT_OUTPUT_DIR": str(self.output),
            "CARGOKIT_TARGET_TEMP_DIR": str(self.root / "temporary with spaces"),
        }
        for name in ["ORT_LIB_LOCATION", "ORT_LIB_PATH", "ORT_PREFER_DYNAMIC_LINK"]:
            self.env.pop(name, None)

    def build(self, architectures, **overrides):
        return subprocess.run(
            ["bash", str(self.scripts / "build_macos_slices.sh"), "unused", "host_flutter"],
            env={**self.env, "CARGOKIT_DARWIN_ARCHS": architectures, **overrides},
            capture_output=True,
            text=True,
        )

    def tools(self):
        return [json.loads(line) for line in (self.root / "tools.log").read_text().splitlines()]

    def test_universal_build_isolates_runtime_environments(self):
        for architectures in ["arm64 x86_64", "x86_64 arm64"]:
            with self.subTest(architectures=architectures):
                result = self.build(architectures)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(
                    set((self.output / "libhost_flutter.a").read_text().split()),
                    {"arm64", "x86_64"},
                )
                self.assertEqual((self.output / "libwebgpu_dawn.dylib").read_text(), "arm64")
                self.assertEqual((self.output / "libonnxruntime.dylib").read_text(), "x86_64")
        builds = (self.root / "build.log").read_text().splitlines()
        self.assertTrue(all(line == "arm64||" for line in builds if line.startswith("arm64")))
        self.assertTrue(all(line.endswith("/onnxruntime-x86_64|1") for line in builds if line.startswith("x86_64")))
        self.assertEqual(sum(name == "curl" for name, _ in self.tools()), 1)

    def test_single_arch_build_drops_stale_sidecars(self):
        self.assertEqual(self.build("arm64 x86_64").returncode, 0)
        result = self.build("arm64")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.output / "libonnxruntime.dylib").exists())
        result = self.build("x86_64")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.output / "libwebgpu_dawn.dylib").exists())

    def test_bad_checksum_aborts_before_install_or_build(self):
        result = self.build("x86_64", BAD_CHECKSUM="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.root / "build.log").exists())
        self.assertNotIn("install_name_tool", [name for name, _ in self.tools()])
        self.assertFalse((self.output / "libonnxruntime.dylib").exists())

    def test_missing_slice_aborts_instead_of_publishing_partial_archive(self):
        result = self.build("arm64 x86_64", FAIL_ARCH="x86_64")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.output / "libhost_flutter.a").exists())

    def test_invalid_architectures_fail(self):
        for architectures in ["", "i386"]:
            with self.subTest(architectures=architectures):
                self.assertNotEqual(self.build(architectures).returncode, 0)


if __name__ == "__main__":
    unittest.main()
