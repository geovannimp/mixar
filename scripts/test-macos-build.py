#!/usr/bin/env python3
"""Exercise macOS slice orchestration without Xcode or a network connection."""

import hashlib
import io
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tempfile
import textwrap
import unittest


ROOT = Path(__file__).resolve().parents[1]
SCRIPTS = ROOT / "apps/gui-flutter/rust_builder/cargokit"
ORT_RELEASE_SHA256 = "d10359e16347b57d9959f7e80a225a5b4a66ed7d7e007274a15cae86836485a6"

# Fake native tools model architectures as file contents. The real macOS release
# job additionally checks the built Mach-O slices and executable load commands.
TOOLS = r'''#!/usr/bin/env python3
import json, os, pathlib, re, shutil, subprocess, sys
name = pathlib.Path(sys.argv[0]).name
args = sys.argv[1:]
with open(os.environ['TOOL_LOG'], 'a') as log:
    log.write(json.dumps([name, args]) + '\n')
if name == 'curl':
    url = next((a for a in args if a.startswith('http')), '')
    output = args[args.index('--output') + 1]
    primary = 'github.com' in url
    # Microsoft's Intel macOS asset 404s; make the stub reproduce that by
    # default so the fallback path is what every build exercises.
    reachable = os.environ.get('ORT_PRIMARY_OK') == '1' if primary else True
    if os.environ.get('ORT_ALL_DOWNLOADS_FAIL') == '1':
        reachable = False
    with open(os.environ['TOOL_LOG'], 'a') as log:
        log.write(json.dumps(['download-ok' if reachable else 'download-404', args]) + '\n')
    if not reachable:
        print('curl: (56) The requested URL returned error: 404', file=sys.stderr)
        sys.exit(56)
    shutil.copyfile(os.environ['ORT_ARCHIVE'], output)
elif name == 'shasum':
    assert args == ['-a', '256', '--check'], args
    line = sys.stdin.read().rstrip('\n')
    match = re.fullmatch(r'([0-9a-f]{64})  (.+)', line)
    assert match is not None, line
    assert match.group(1) == os.environ['FIXTURE_SHA256'], line
    assert pathlib.Path(match.group(2)).is_file(), line
    if os.environ.get('BAD_CHECKSUM') == '1':
        line = '0' * 64 + '  ' + match.group(2)
    # Exercise the real SHA-256 checker, including its failure path.
    result = subprocess.run([os.environ['REAL_SHASUM'], *args], input=line + '\n', text=True)
    sys.exit(result.returncode)
elif name == 'install_name_tool':
    assert args[:2] == ['-id', '@rpath/libonnxruntime.dylib']
elif name == 'otool':
    print('@rpath/libonnxruntime.dylib')
    print('@rpath/libwebgpu_dawn.dylib')
elif name == 'lipo':
    if args[0] == '-create':
        output = args.index('-output')
        contents = ' '.join(pathlib.Path(p).read_text() for p in args[1:output])
        pathlib.Path(args[output + 1]).write_text(contents)
    elif '-verify_arch' in args:
        # lipo(1): "all of the input files must appear before the -verify_arch
        # flag". The real tool rejects the reversed form with this message, and
        # the stub has to do the same or it hides that regression.
        flag = args.index('-verify_arch')
        files, arch_flags = args[:flag], args[flag + 1:]
        known = {'arm64', 'arm64e', 'x86_64', 'x86_64h'}
        if not files or not arch_flags or any(f.startswith('-') for f in files):
            unknown = next((a for a in files + arch_flags if a not in known), '')
            print(
                f'lipo: unknown architecture specification flag: {unknown} '
                'in specifying -verify_arch operation',
                file=sys.stderr,
            )
            sys.exit(1)
        present = set()
        for path in files:
            present.update(pathlib.Path(path).read_text().split())
        sys.exit(0 if all(a in present for a in arch_flags) else 1)
    else:
        print(f'unexpected lipo invocation: {args}', file=sys.stderr)
        sys.exit(1)
'''

BUILD_TOOL = '''#!/bin/sh
set -eu
printf '%s|%s|%s\\n' "$CARGOKIT_DARWIN_ARCHS" "${ORT_LIB_LOCATION:-}" "${ORT_PREFER_DYNAMIC_LINK:-}" >> "$BUILD_LOG"
if [ "${FAIL_ARCH:-}" = "$CARGOKIT_DARWIN_ARCHS" ]; then exit 1; fi
printf '%s' "$CARGOKIT_DARWIN_ARCHS" > "$CARGOKIT_OUTPUT_DIR/libhost_flutter.a"
if [ "$CARGOKIT_DARWIN_ARCHS" = arm64 ]; then
  if [ "${OMIT_DAWN:-}" != 1 ]; then
    printf '%s' "${DAWN_ARCH:-arm64}" > "$CARGOKIT_OUTPUT_DIR/libwebgpu_dawn.dylib"
  fi
  if [ "${CROSS_ARCH_SIDECARS:-}" = 1 ]; then
    printf x86_64 > "$CARGOKIT_OUTPUT_DIR/libonnxruntime.dylib"
  fi
elif [ "${CROSS_ARCH_SIDECARS:-}" = 1 ]; then
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
        for name in ["build_macos_slices.sh", "prepare_macos_ort.sh", "build_pod.sh"]:
            shutil.copyfile(SCRIPTS / name, self.scripts / name)
        (self.scripts / "run_build_tool.sh").write_text(BUILD_TOOL)
        tools = self.root / "tools"
        tools.mkdir()
        for name in ["curl", "shasum", "install_name_tool", "lipo", "otool"]:
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
        prepare = self.scripts / "prepare_macos_ort.sh"
        source = prepare.read_text()
        pin = re.search(r"^checksum=([0-9a-f]{64})$", source, re.MULTILINE)
        self.assertIsNotNone(pin, "The runtime pin must be a SHA-256 digest")
        self.assertEqual(pin.group(1), ORT_RELEASE_SHA256)
        # A tiny local fixture cannot hash to the official release's digest.
        # Assert the production pin above, then replace only the copied script's
        # pin so the real shasum command can verify our offline fixture.
        fixture_checksum = hashlib.sha256(archive.read_bytes()).hexdigest()
        prepare.write_text(source.replace(ORT_RELEASE_SHA256, fixture_checksum, 1))
        real_shasum = shutil.which("shasum")
        self.assertIsNotNone(real_shasum, "Packaging tests require shasum")
        self.output = self.root / "output with spaces"
        self.manifest = self.root / "manifest"
        self.manifest.mkdir()
        self.env = {
            **os.environ,
            "PATH": str(tools) + os.pathsep + os.environ["PATH"],
            "TOOL_LOG": str(self.root / "tools.log"),
            "BUILD_LOG": str(self.root / "build.log"),
            "ORT_ARCHIVE": str(archive),
            "FIXTURE_SHA256": fixture_checksum,
            "REAL_SHASUM": real_shasum,
            "CARGOKIT_OUTPUT_DIR": str(self.output),
            "CARGOKIT_TARGET_TEMP_DIR": str(self.root / "temporary with spaces"),
            "CARGOKIT_MANIFEST_DIR": str(self.manifest),
        }
        for name in [
            "ORT_LIB_LOCATION",
            "ORT_LIB_PATH",
            "ORT_PREFER_DYNAMIC_LINK",
            "PLATFORM_NAME",
        ]:
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

    def verified_architectures(self, matches):
        """Architectures this run verified on artifacts whose path matches."""
        verified = []
        for name, args in self.tools():
            if name != "lipo" or "-verify_arch" not in args:
                continue
            flag = args.index("-verify_arch")
            if any(matches(path) for path in args[:flag]):
                verified.extend(args[flag + 1:])
        return verified

    def test_merged_archive_is_verified_for_every_architecture(self):
        self.assertEqual(self.build("arm64 x86_64").returncode, 0)
        merged = str(self.output / "libhost_flutter.a")
        # Without this the suite stays green when the final verification is
        # deleted, because nothing else observes the merged archive.
        self.assertEqual(
            sorted(self.verified_architectures(lambda path: path == merged)),
            ["arm64", "x86_64"],
        )

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
        # The runtime is downloaded once and reused from the target temp dir.
        self.assertEqual(sum(name == "download-ok" for name, _ in self.tools()), 1)

    def test_cross_architecture_sidecars_are_ignored_in_both_build_orders(self):
        for architectures in ["arm64 x86_64", "x86_64 arm64"]:
            with self.subTest(architectures=architectures):
                result = self.build(architectures, CROSS_ARCH_SIDECARS="1")
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual((self.output / "libwebgpu_dawn.dylib").read_text(), "arm64")
                self.assertEqual((self.output / "libonnxruntime.dylib").read_text(), "x86_64")

    def test_required_runtime_must_exist_and_match_its_architecture(self):
        for overrides in [{"DAWN_ARCH": "x86_64"}, {"OMIT_DAWN": "1"}]:
            with self.subTest(overrides=overrides):
                result = self.build("arm64", **overrides)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse((self.output / "libhost_flutter.a").exists())
                self.assertFalse((self.output / "libwebgpu_dawn.dylib").exists())

    def test_missing_or_empty_script_arguments_fail_with_usage(self):
        cases = [
            ("build_macos_slices.sh", [[], ["unused"], ["unused", ""], ["", "host_flutter"]]),
            ("build_pod.sh", [[], ["unused"], ["unused", ""], ["", "host_flutter"]]),
            ("prepare_macos_ort.sh", [[], [""]]),
        ]
        for script, argument_sets in cases:
            for arguments in argument_sets:
                with self.subTest(script=script, arguments=arguments):
                    result = subprocess.run(
                        ["bash", str(self.scripts / script), *arguments],
                        env=self.env,
                        capture_output=True,
                        text=True,
                    )
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn("usage:", result.stderr)
        self.assertFalse((self.root / "build.log").exists())

    def test_missing_build_environment_names_the_unset_variable(self):
        cases = [
            ("CARGOKIT_OUTPUT_DIR", None),
            ("CARGOKIT_TARGET_TEMP_DIR", None),
            ("CARGOKIT_MANIFEST_DIR", "missing"),
            ("CARGOKIT_MANIFEST_DIR", "not-a-directory"),
        ]
        for variable, state in cases:
            with self.subTest(variable=variable, state=state):
                environment = dict(self.env)
                if state is None:
                    environment.pop(variable)
                elif state == "missing":
                    environment[variable] = str(self.root / "absent")
                else:
                    environment[variable] = str(self.root / "tools" / "shasum")
                environment["CARGOKIT_DARWIN_ARCHS"] = "arm64"
                result = subprocess.run(
                    [
                        "bash",
                        str(self.scripts / "build_macos_slices.sh"),
                        "unused",
                        "host_flutter",
                    ],
                    env=environment,
                    capture_output=True,
                    text=True,
                )
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(variable, result.stderr)
        self.assertFalse((self.root / "build.log").exists())

    def test_missing_or_unknown_platform_fails_before_dispatch(self):
        for platform in [None, "", "linux", "macos"]:
            with self.subTest(platform=platform):
                environment = dict(self.env)
                environment.pop("PLATFORM_NAME", None)
                if platform is not None:
                    environment["PLATFORM_NAME"] = platform
                result = subprocess.run(
                    ["bash", str(self.scripts / "build_pod.sh"), "unused", "host_flutter"],
                    env=environment,
                    capture_output=True,
                    text=True,
                )
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("unsupported or missing PLATFORM_NAME", result.stderr)
        self.assertFalse((self.root / "build.log").exists())

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

    def download_attempts(self):
        """(outcome, url) for each curl attempt, in order."""
        return [
            (name, next(a for a in args if a.startswith("http")))
            for name, args in self.tools()
            if name.startswith("download-")
        ]

    def assert_first_download_is_canonical(self):
        """The Microsoft release is always tried before any mirror."""
        url = self.download_attempts()[0][1]
        self.assertIn("github.com/microsoft/onnxruntime", url)

    def test_runtime_download_falls_back_when_the_release_asset_is_gone(self):
        # The stub 404s every github.com asset by default, which is what
        # Microsoft's last Intel macOS release does today.
        result = self.build("x86_64")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            [outcome for outcome, _ in self.download_attempts()],
            ["download-404", "download-ok"],
        )
        self.assert_first_download_is_canonical()
        # The mirror copy still has to match the pinned Microsoft digest.
        self.assertEqual((self.output / "libonnxruntime.dylib").read_text(), "x86_64")

    def test_runtime_download_prefers_the_canonical_release(self):
        result = self.build("x86_64", ORT_PRIMARY_OK="1")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            [outcome for outcome, _ in self.download_attempts()],
            ["download-ok"],
        )
        self.assert_first_download_is_canonical()
        self.assertNotIn("photoprism", result.stderr)

    def test_runtime_download_failure_names_every_source(self):
        result = self.build("x86_64", ORT_ALL_DOWNLOADS_FAIL="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("could not download ONNX Runtime", result.stderr)
        self.assertIn("github.com/microsoft/onnxruntime", result.stderr)
        self.assertIn("photoprism", result.stderr)
        self.assertFalse((self.root / "build.log").exists())
        self.assertFalse((self.output / "libonnxruntime.dylib").exists())

    def test_checksum_algorithm_and_arguments_are_verified(self):
        prepare = self.scripts / "prepare_macos_ort.sh"
        prepare.write_text(prepare.read_text().replace(
            "shasum -a 256 --check", "shasum -a 512 --check", 1
        ))
        result = self.build("x86_64")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.root / "build.log").exists())

    def test_missing_slice_aborts_instead_of_publishing_partial_archive(self):
        result = self.build("arm64 x86_64", FAIL_ARCH="x86_64")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.output / "libhost_flutter.a").exists())

    def test_invalid_architectures_fail(self):
        for architectures in ["", "i386"]:
            with self.subTest(architectures=architectures):
                self.assertNotEqual(self.build(architectures).returncode, 0)

    @staticmethod
    def release_script():
        workflow = (ROOT / ".github/workflows/release.yml").read_text()
        section = workflow.split("      - name: Verify universal macOS app\n", 1)[1]
        section = section.split("\n      - name:", 1)[0]
        return textwrap.dedent(section.split("        run: |\n", 1)[1])

    def release_app(self, *, embedded_loader=False):
        """Build the Release bundle layout the verification step inspects."""
        app = self.root / "build/macos/Build/Products/Release/Mixar.app/Contents"
        (app / "MacOS").mkdir(parents=True)
        (app / "MacOS/Mixar").write_text("arm64 x86_64")
        frameworks = app / "Frameworks"
        frameworks.mkdir()
        (frameworks / "libwebgpu_dawn.dylib").write_text("arm64")
        (frameworks / "libonnxruntime.dylib").write_text("x86_64")
        if embedded_loader:
            loader = frameworks / "host_flutter.framework/Versions/A/host_flutter"
            loader.parent.mkdir(parents=True)
            loader.write_text("arm64 x86_64")
        return app

    def run_release_script(self):
        return subprocess.run(
            ["bash", "-c", self.release_script()],
            cwd=self.root,
            env=self.env,
            capture_output=True,
            text=True,
        )

    def test_release_validation_names_missing_runtimes_and_allows_runner_fallback(self):
        frameworks = self.release_app() / "Frameworks"
        runtimes = {
            "libwebgpu_dawn.dylib": "arm64",
            "libonnxruntime.dylib": "x86_64",
        }
        # No host_flutter.framework: that optional lookup must use Runner.
        for missing in [*runtimes, None]:
            with self.subTest(missing=missing):
                for name, architecture in runtimes.items():
                    library = frameworks / name
                    if name == missing:
                        library.unlink(missing_ok=True)
                    else:
                        library.write_text(architecture)
                result = self.run_release_script()
                if missing is None:
                    self.assertEqual(result.returncode, 0, result.stderr)
                    verified = set(
                        self.verified_architectures(
                            lambda path: path.endswith("MacOS/Mixar")
                        )
                    )
                    self.assertEqual(verified, {"arm64", "x86_64"})
                else:
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn(f"{missing} not found under", result.stderr)

    def test_release_validation_verifies_the_embedded_rust_loader(self):
        self.release_app(embedded_loader=True)

        result = self.run_release_script()
        self.assertEqual(result.returncode, 0, result.stderr)
        # HOST only takes this branch when the framework exists, so deleting
        # its loop must fail here even though Runner falls back elsewhere.
        self.assertEqual(
            sorted(
                self.verified_architectures(
                    lambda path: path.endswith(
                        "host_flutter.framework/Versions/A/host_flutter"
                    )
                )
            ),
            ["arm64", "x86_64"],
        )


if __name__ == "__main__":
    unittest.main()
