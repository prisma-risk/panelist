# SPDX-License-Identifier: Apache-2.0
"""Release preparation contracts, without publishing or changing the source tree."""
import importlib.util
import io
from pathlib import Path
import subprocess
import tarfile
import tempfile
import tomllib
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("release_package", Path(__file__).with_name("release_package.py"))
RELEASE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RELEASE)


class ReleasePackageTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="panelist-release-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / "Cargo.toml").write_text('[workspace]\nmembers=["crates/panelist"]\n[workspace.package]\nversion = "0.3.0"\npublish=true\n')
        (self.root / "Cargo.lock").write_text('version=4\n[[package]]\nname = "panelist"\nversion = "0.3.0"\n[[package]]\nname="serde"\nversion="1.0.0"\nsource="registry+https://github.com/rust-lang/crates.io-index"\nchecksum="unchanged"\n')

    def test_selected_version_packages_offline_and_preserves_locked_dependencies(self):
        original = tomllib.loads((self.root / "Cargo.lock").read_text())["package"][1]

        def package(command, cwd, check):
            self.assertEqual(command, ["cargo", "package", "--locked", "--allow-dirty", "--workspace", "--registry", "crates-io"])
            self.assertEqual(cwd, self.root)
            self.assertTrue(check)
            archive = self.root / "target/package/panelist-0.4.0.crate"
            archive.parent.mkdir(parents=True, exist_ok=True)
            with tarfile.open(archive, "w:gz") as output:
                content = b'[package]\nname="panelist"\nversion="0.4.0"\n'
                entry = tarfile.TarInfo("panelist-0.4.0/Cargo.toml")
                entry.size = len(content)
                output.addfile(entry, io.BytesIO(content))

        with patch.object(subprocess, "run", side_effect=package), patch.object(subprocess, "check_output", return_value="a" * 40):
            RELEASE.prepare(self.root, "0.4.0", "panelist")
            first = (self.root / "target/release-bundle/metadata.tar.gz").read_bytes()
            RELEASE.prepare(self.root, "0.4.0", "panelist")
        self.assertEqual(tomllib.loads((self.root / "Cargo.lock").read_text())["package"][1], original)
        self.assertEqual(first, (self.root / "target/release-bundle/metadata.tar.gz").read_bytes())
        with tarfile.open(fileobj=io.BytesIO(first)) as metadata:
            self.assertEqual(metadata.getnames(), ["Cargo.lock", "Cargo.toml"])
            self.assertEqual(tomllib.loads(metadata.extractfile("Cargo.toml").read().decode())["workspace"]["package"]["version"], "0.4.0")

    def test_lockstep_macros_are_prepared_and_packaged_before_the_primary_crate(self):
        manifest = self.root / "Cargo.toml"
        manifest.write_text(manifest.read_text().replace('["crates/panelist"]', '["crates/panelist", "crates/panelist-macros"]') +
                            '[workspace.dependencies]\npanelist-macros = { version = "=0.3.0", path = "crates/panelist-macros" }\n')
        lock = self.root / "Cargo.lock"
        lock.write_text(lock.read_text() + '[[package]]\nname = "panelist-macros"\nversion = "0.3.0"\n')

        def package(command, cwd, check):
            for name in ("panelist-macros", "panelist"):
                archive = self.root / f"target/package/{name}-0.4.0.crate"
                archive.parent.mkdir(parents=True, exist_ok=True)
                with tarfile.open(archive, "w:gz") as output:
                    content = f'[package]\nname="{name}"\nversion="0.4.0"\n'.encode()
                    entry = tarfile.TarInfo(f"{name}-0.4.0/Cargo.toml")
                    entry.size = len(content)
                    output.addfile(entry, io.BytesIO(content))

        with patch.object(subprocess, "run", side_effect=package), patch.object(subprocess, "check_output", return_value="a" * 40):
            RELEASE.prepare(self.root, "0.4.0", "panelist")
        import json
        proof = json.loads((self.root / "target/release-bundle/provenance.json").read_text())
        self.assertEqual([p["package"] for p in proof["distributions"]], ["panelist-macros", "panelist"])
        self.assertEqual(tomllib.loads(manifest.read_text())["workspace"]["dependencies"]["panelist-macros"]["version"], "=0.4.0")
        self.assertEqual([p["version"] for p in tomllib.loads(lock.read_text())["package"] if p["name"].startswith("panelist")], ["0.4.0", "0.4.0"])

    def test_invalid_versions_package_and_private_admission_have_no_effects(self):
        for version, package in [("v0.4.0", "panelist"), ("0.4.0-rc.1", "panelist"), ("00.4.0", "panelist"), ("0.0.0", "panelist"), ("0.4.0", "foreign")]:
            before = (self.root / "Cargo.toml").read_bytes()
            with self.assertRaises(ValueError), patch.object(subprocess, "run") as run:
                RELEASE.prepare(self.root, version, package)
            run.assert_not_called()
            self.assertEqual(before, (self.root / "Cargo.toml").read_bytes())
        manifest = self.root / "Cargo.toml"
        manifest.write_text(manifest.read_text().replace("publish=true", "publish=['example-private']"))
        with self.assertRaises(ValueError), patch.object(subprocess, "run") as run:
            RELEASE.prepare(self.root, "0.4.0", "panelist")
        run.assert_not_called()

    def test_mismatched_lock_refuses_before_changing_manifest(self):
        lock = self.root / "Cargo.lock"
        lock.write_text(lock.read_text().replace('version = "0.3.0"', 'version = "0.2.0"'))
        before = (self.root / "Cargo.toml").read_bytes()
        with self.assertRaises(ValueError), patch.object(subprocess, "run") as run:
            RELEASE.prepare(self.root, "0.4.0", "panelist")
        run.assert_not_called()
        self.assertEqual(before, (self.root / "Cargo.toml").read_bytes())


if __name__ == "__main__":
    unittest.main()
