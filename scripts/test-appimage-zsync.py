#!/usr/bin/env python3
"""Regression tests for the release-side AppImage zsync plumbing.

The production behaviour lives in `.github/workflows/release.yml`: a wrapper
that injects `--updateinformation` into appimagetool before Fastforge runs, and
a step that pairs a finished AppImage with its `.zsync` sidecar. Both shell
scripts are extracted straight from the workflow and executed here against
throwaway fixtures, so this suite fails if the workflow and its contract drift
apart (for example if appimagetool output moves or the sidecar stops matching
the image).

Run with: python3 scripts/test-appimage-zsync.py
Requires PyYAML to read the workflow; everything else is stdlib.
"""

import hashlib
import json
import os
import shutil
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW_PATH = ROOT / ".github/workflows/release.yml"
ACTION_PATH = ROOT / ".github/actions/install-flutter-desktop-deps/action.yml"
BASH = shutil.which("bash") or "/bin/bash"

# Utilities the preparation script itself needs; a sandboxed PATH has to carry
# them so that only the tool under test can be the reason for a failure.
SCRIPT_TOOLS = ("realpath", "mkdir", "cat", "chmod")

REPOSITORY = "geovannimp/mixar"
UPDATE_INFORMATION = "gh-releases-zsync|geovannimp|mixar|latest|*.AppImage.zsync"

PREPARE_STEP = "Prepare AppImage update information"
VERIFY_STEP = "Verify AppImage zsync artifacts"

FAKE_APPIMAGETOOL = """\
#!/usr/bin/env bash
set -euo pipefail
python3 - "$FAKE_ARGV_JSON" "$@" <<'PYJSON'
import json
import sys

with open(sys.argv[1], "w", encoding="utf-8") as handle:
    json.dump(sys.argv[2:], handle)
PYJSON
exit "${FAKE_ARGV_EXIT:-0}"
"""

FAKE_ZSYNCMAKE = """\
#!/usr/bin/env bash
set -euo pipefail
echo "zsyncmake 0.6.2 (fake)"
"""


def load_workflow():
    with WORKFLOW_PATH.open(encoding="utf-8") as handle:
        return yaml.safe_load(handle)


def workflow_step(name):
    """Return the build-job step named `name`, scripts included."""
    for step in load_workflow()["jobs"]["build"]["steps"]:
        if step.get("name") == name:
            run = step.get("run")
            if not isinstance(run, str) or not run.strip():
                raise AssertionError("workflow step %r has no run script" % name)
            return step
    raise AssertionError("workflow has no build step named %r" % name)


def build_env(tmp, tools, repository=REPOSITORY, base=None):
    """Emulate the runner environment the two steps see."""
    env = dict(os.environ if base is None else base)
    env.pop("UPDATE_INFORMATION", None)
    env.pop("MIXAR_APPIMAGETOOL_REAL", None)
    (tmp / "github_env").write_text("", encoding="utf-8")
    (tmp / "github_path").write_text("", encoding="utf-8")
    (tmp / "runner_temp").mkdir(exist_ok=True)
    env.update(
        {
            "PATH": "%s:%s" % (tools, env.get("PATH", "/usr/bin:/bin")),
            "RUNNER_TEMP": str(tmp / "runner_temp"),
            "GITHUB_ENV": str(tmp / "github_env"),
            "GITHUB_PATH": str(tmp / "github_path"),
            "GITHUB_REPOSITORY": repository,
        }
    )
    return env


def apply_github_files(tmp, env):
    """Fold GITHUB_ENV/GITHUB_PATH back in, as Actions does for later steps."""
    assignments = {}
    for line in (tmp / "github_env").read_text(encoding="utf-8").splitlines():
        if line.strip():
            key, _, value = line.partition("=")
            assignments[key.strip()] = value
    entries = [
        line.strip()
        for line in (tmp / "github_path").read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    merged = dict(env)
    merged.update(assignments)
    merged["PATH"] = ":".join(entries + [env["PATH"]])
    return assignments, entries, merged


def write_tool(path, body):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(textwrap.dedent(body), encoding="utf-8")
    path.chmod(0o755)
    return path


def run_bash(script, cwd, env):
    # Absolute shell path: a sandboxed PATH deliberately omits /bin.
    return subprocess.run(
        [BASH, "-c", script],
        cwd=str(cwd),
        env=env,
        capture_output=True,
        text=True,
    )


def sandbox_tools(tmp):
    """A PATH holding appimagetool and only the shell tools the script needs."""
    sandbox = tmp / "sandbox"
    sandbox.mkdir()
    for name in SCRIPT_TOOLS:
        target = shutil.which(name)
        if target is None:
            raise unittest.SkipTest("host is missing %s" % name)
        (sandbox / name).symlink_to(target)
    write_tool(sandbox / "appimagetool", FAKE_APPIMAGETOOL)
    return sandbox


class WorkflowShapeTest(unittest.TestCase):
    """The published asset set must travel through both release stages."""

    def setUp(self):
        workflow = load_workflow()
        matrix = workflow["jobs"]["build"]["strategy"]["matrix"]["include"]
        self.by_platform = {entry["platform"]: entry for entry in matrix}
        self.publish_steps = workflow["jobs"]["publish"]["steps"]

    def test_linux_artifact_pattern_ships_image_and_sidecar(self):
        pattern = self.by_platform["linux"]["artifact_pattern"]
        lines = [line.strip() for line in pattern.splitlines() if line.strip()]
        self.assertEqual(
            lines,
            [
                "apps/gui-flutter/dist/**/*.AppImage",
                "apps/gui-flutter/dist/**/*.AppImage.zsync",
            ],
        )

    def test_other_platform_patterns_unchanged(self):
        self.assertEqual(
            self.by_platform["macos"]["artifact_pattern"],
            "apps/gui-flutter/dist/**/*.dmg",
        )
        self.assertEqual(
            self.by_platform["windows"]["artifact_pattern"],
            "apps/gui-flutter/dist/**/*.exe",
        )

    def test_installer_upload_keeps_pattern_and_fails_when_empty(self):
        step = next(
            s
            for s in load_workflow()["jobs"]["build"]["steps"]
            if s.get("name") == "Upload installer artifact"
        )
        self.assertEqual(step["with"]["path"], "${{ matrix.artifact_pattern }}")
        self.assertEqual(step["with"]["if-no-files-found"], "error")

    def test_draft_release_publishes_sidecar(self):
        step = next(
            s for s in self.publish_steps if s.get("name") == "Create draft GitHub Release"
        )
        lines = [line.strip() for line in step["with"]["files"].splitlines() if line.strip()]
        self.assertEqual(
            lines,
            [
                "release-assets/**/*.AppImage",
                "release-assets/**/*.AppImage.zsync",
                "release-assets/**/*.dmg",
                "release-assets/**/*.exe",
            ],
        )
        self.assertIs(step["with"]["draft"], True)
        self.assertEqual(
            step["with"]["prerelease"], "${{ steps.tag.outputs.prerelease == 'true' }}"
        )

    def test_linux_deps_install_zsyncmake(self):
        with ACTION_PATH.open(encoding="utf-8") as handle:
            action = yaml.safe_load(handle)
        linux = next(
            s for s in action["runs"]["steps"] if s.get("name") == "Linux Flutter + AppImage tools"
        )
        self.assertIn("zsync", linux["run"])
        self.assertIn("command -v zsyncmake", linux["run"])
        self.assertIn("APPIMAGETOOL_SHA256", linux["env"])


class PrepareStepTest(unittest.TestCase):
    """The wrapper injects update information and forwards everything else."""

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.tmp = Path(self._tmp.name)
        self.tools = self.tmp / "tools"
        self.real_tool = write_tool(self.tools / "appimagetool", FAKE_APPIMAGETOOL)
        write_tool(self.tools / "zsyncmake", FAKE_ZSYNCMAKE)
        self.script = workflow_step(PREPARE_STEP)["run"]
        self.argv_json = self.tmp / "argv.json"

    def prepare(self, **kwargs):
        env = build_env(self.tmp, self.tools, **kwargs)
        result = run_bash(self.script, self.tmp, env)
        return result, apply_github_files(self.tmp, env)

    def test_records_env_and_path_expectations(self):
        result, (assignments, entries, _) = self.prepare()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            assignments["MIXAR_APPIMAGETOOL_REAL"], str(self.real_tool.resolve())
        )
        self.assertEqual(assignments["UPDATE_INFORMATION"], UPDATE_INFORMATION)
        self.assertEqual(
            entries, [str(self.tmp / "runner_temp" / "mixar-appimage-tools")]
        )

    def test_wrapper_prepends_update_information_once(self):
        result, (_, _, env) = self.prepare()
        self.assertEqual(result.returncode, 0, result.stderr)
        env["FAKE_ARGV_JSON"] = str(self.argv_json)
        env.pop("FAKE_ARGV_EXIT", None)
        appdir = "App Dir with spaces/appdir"
        output = "out dir/My App-1.0.AppImage"
        result = subprocess.run(
            ["appimagetool", "--no-appstream", appdir, output],
            env=env,
            cwd=str(self.tmp),
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            json.loads(self.argv_json.read_text(encoding="utf-8")),
            ["--updateinformation", UPDATE_INFORMATION, "--no-appstream", appdir, output],
        )

    def test_wrapper_propagates_real_tool_failure(self):
        result, (_, _, env) = self.prepare()
        self.assertEqual(result.returncode, 0, result.stderr)
        env["FAKE_ARGV_JSON"] = str(self.argv_json)
        env["FAKE_ARGV_EXIT"] = "23"
        result = subprocess.run(
            ["appimagetool", "--no-appstream", "AppDir", "out.AppImage"],
            env=env,
            cwd=str(self.tmp),
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 23, result.stderr)

    def test_wrapper_requires_update_information(self):
        result, (_, _, env) = self.prepare()
        self.assertEqual(result.returncode, 0, result.stderr)
        env["FAKE_ARGV_JSON"] = str(self.argv_json)
        env.pop("UPDATE_INFORMATION", None)
        result = subprocess.run(
            ["appimagetool", "--no-appstream", "AppDir", "out.AppImage"],
            env=env,
            cwd=str(self.tmp),
            capture_output=True,
            text=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(
            self.argv_json.exists(), "wrapper must not reach the real tool"
        )

    def test_rejects_malformed_repository(self):
        for repository in ("invalid", "owner/", "/repo", "a/b/c"):
            with self.subTest(repository=repository):
                result, _ = self.prepare(repository=repository)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("owner/repo", result.stderr)

    def test_rejects_missing_zsyncmake(self):
        # A host that already ships zsyncmake (the very machines this feature
        # targets) must not mask the case, so the step runs against a PATH that
        # holds only itself and appimagetool.
        sandbox = sandbox_tools(self.tmp)
        zsyncmake = write_tool(sandbox / "zsyncmake", FAKE_ZSYNCMAKE)
        env = build_env(self.tmp, sandbox, base={"PATH": str(sandbox)})
        result = run_bash(self.script, self.tmp, env)
        self.assertEqual(result.returncode, 0, result.stderr)

        zsyncmake.unlink()
        env = build_env(self.tmp, sandbox, base={"PATH": str(sandbox)})
        result = run_bash(self.script, self.tmp, env)
        self.assertNotEqual(result.returncode, 0)
        # It stopped at the tool gate: nothing was handed to the packager.
        self.assertEqual((self.tmp / "github_env").read_text(encoding="utf-8"), "")


class VerifyStepTest(unittest.TestCase):
    """Only an image whose sidecar and metadata agree may reach publication."""

    IMAGE_NAME = "gui_flutter-1.0-linux.AppImage"

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.cwd = Path(self._tmp.name)
        self.script = workflow_step(VERIFY_STEP)["run"]

    def make_image(self, reported=UPDATE_INFORMATION, name=None):
        image_dir = self.cwd / "dist" / "1.0"
        image_dir.mkdir(parents=True, exist_ok=True)
        image = image_dir / (name or self.IMAGE_NAME)
        image.write_text(
            "#!/usr/bin/env bash\n"
            "set -euo pipefail\n"
            "if [ \"${1:-}\" = '--appimage-updateinformation' ]; then\n"
            "  printf '%s\\n' " + json.dumps(reported) + "\n"
            "  exit 0\n"
            "fi\n"
            "echo 'unexpected invocation' >&2\n"
            "exit 1\n",
            encoding="utf-8",
        )
        image.chmod(0o755)
        return image

    def make_sidecar(self, image, **overrides):
        data = image.read_bytes()
        fields = {
            "zsync": "0.6.2",
            "Filename": image.name,
            "URL": image.name,
            "Length": str(len(data)),
            "SHA-1": hashlib.sha1(data).hexdigest(),
        }
        fields.update(overrides)
        text = "".join("%s: %s\n" % (key, value) for key, value in fields.items())
        # Binary, non-ASCII body past the header: never decoded, never inspected.
        return text.encode("ascii") + b"\n\x00\xff\x80\xc3\xa9not-a-header\x00"

    def write_sidecar(self, path, payload):
        path.write_bytes(payload)
        return path

    def verify(self, update_information=UPDATE_INFORMATION):
        env = dict(os.environ)
        env["UPDATE_INFORMATION"] = update_information
        return run_bash(self.script, self.cwd, env)

    def test_sidecar_beside_image_passes(self):
        image = self.make_image()
        sidecar = self.write_sidecar(
            image.with_name(image.name + ".zsync"), self.make_sidecar(image)
        )
        result = self.verify()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("dist/1.0/" + image.name, result.stdout)
        self.assertIn("dist/1.0/" + sidecar.name, result.stdout)

    def test_sidecar_in_working_directory_is_relocated(self):
        image = self.make_image()
        detached = self.write_sidecar(
            self.cwd / (image.name + ".zsync"), self.make_sidecar(image)
        )
        result = self.verify()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(image.with_name(image.name + ".zsync").is_file())
        self.assertFalse(detached.exists())

    def test_missing_sidecar_fails(self):
        self.make_image()
        result = self.verify()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("missing sidecar", result.stderr)

    def test_ambiguous_sidecars_fail_without_overwriting(self):
        image = self.make_image()
        beside = self.write_sidecar(
            image.with_name(image.name + ".zsync"), self.make_sidecar(image)
        )
        detached = self.write_sidecar(
            self.cwd / (image.name + ".zsync"), self.make_sidecar(image)
        )
        beside_payload = beside.read_bytes()
        detached_payload = detached.read_bytes()
        result = self.verify()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("ambiguous", result.stderr)
        self.assertEqual(beside.read_bytes(), beside_payload)
        self.assertEqual(detached.read_bytes(), detached_payload)

    def test_wrong_embedded_update_information_fails(self):
        image = self.make_image(reported="gh-releases-zsync|someone|else|latest|*")
        self.write_sidecar(image.with_name(image.name + ".zsync"), self.make_sidecar(image))
        result = self.verify()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("embedded update information", result.stderr)

    def test_wrong_url_fails(self):
        image = self.make_image()
        self.write_sidecar(
            image.with_name(image.name + ".zsync"),
            self.make_sidecar(image, URL="other.AppImage"),
        )
        result = self.verify()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("URL", result.stderr)

    def test_wrong_length_fails(self):
        image = self.make_image()
        self.write_sidecar(
            image.with_name(image.name + ".zsync"),
            self.make_sidecar(image, Length=str(image.stat().st_size + 1)),
        )
        result = self.verify()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Length", result.stderr)

    def test_wrong_sha1_fails(self):
        image = self.make_image()
        self.write_sidecar(
            image.with_name(image.name + ".zsync"),
            self.make_sidecar(image, **{"SHA-1": "0" * 40}),
        )
        result = self.verify()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("SHA-1", result.stderr)

    def test_empty_image_set_fails(self):
        (self.cwd / "dist").mkdir()
        result = self.verify()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("no AppImage", result.stderr)

    def test_two_images_fail(self):
        first = self.make_image()
        self.write_sidecar(
            first.with_name(first.name + ".zsync"), self.make_sidecar(first)
        )
        second = self.make_image(name="gui_flutter-1.0-linux-arm64.AppImage")
        self.write_sidecar(
            second.with_name(second.name + ".zsync"), self.make_sidecar(second)
        )
        result = self.verify()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("exactly one AppImage", result.stderr)


if __name__ == "__main__":
    unittest.main()
