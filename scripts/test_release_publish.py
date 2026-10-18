# SPDX-License-Identifier: Apache-2.0
"""Publication refusal and recovery without any external writes."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("release_publish", Path(__file__).with_name("release_publish.py"))
RELEASE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RELEASE)


class PublicationTests(unittest.TestCase):
    def test_registry_requires_same_unyanked_checksum_and_actual_archive(self):
        content = b"archive"
        digest = hashlib.sha256(content).hexdigest()
        row = {"name": "panelist", "vers": "0.4.0", "yanked": False, "cksum": digest}
        with patch.object(RELEASE, "request", side_effect=[json.dumps(row).encode(), content]) as request:
            self.assertTrue(RELEASE.registry_state("0.4.0", digest))
        self.assertEqual(request.call_count, 2)
        for changes in ({"yanked": True}, {"cksum": "a" * 64}, {"name": "other"}):
            with patch.object(RELEASE, "request", return_value=json.dumps(row | changes).encode()) as request:
                with self.assertRaises(ValueError):
                    RELEASE.registry_state("0.4.0", digest)
                self.assertEqual(request.call_count, 1)
        with patch.object(RELEASE, "request", side_effect=[json.dumps(row).encode(), b"changed"]):
            with self.assertRaises(ValueError):
                RELEASE.registry_state("0.4.0", digest)

    def test_existing_identical_version_has_no_upload(self):
        with patch.object(RELEASE, "registry_state", return_value=True), patch.object(RELEASE, "request") as request:
            RELEASE.publish_crate("0.4.0", "digest", {}, b"archive")
        request.assert_not_called()

    def test_upload_uses_cargo_framing_and_does_not_repeat_uncertain_write(self):
        metadata = {"name": "panelist", "vers": "0.4.0"}
        with patch.dict(os.environ, {"CARGO_REGISTRY_TOKEN": "synthetic"}), patch.object(RELEASE, "registry_state", side_effect=[False, False, True]), patch.object(RELEASE.time, "sleep"), patch.object(RELEASE, "request", side_effect=ValueError("uncertain")) as request:
            RELEASE.publish_crate("0.4.0", "digest", metadata, b"archive")
        request.assert_called_once()
        call = request.call_args
        self.assertEqual(call.args, ("https://crates.io/api/v1/crates/new",))
        self.assertEqual(call.kwargs["method"], "PUT")
        frame = call.kwargs["payload"]
        size, = struct.unpack("<I", frame[:4])
        self.assertEqual(json.loads(frame[4:4 + size]), metadata)
        length, = struct.unpack("<I", frame[4 + size:8 + size])
        self.assertEqual(frame[8 + size:], b"archive")
        self.assertEqual(length, 7)

    def test_unobserved_upload_never_succeeds_or_retries_put(self):
        with patch.dict(os.environ, {"CARGO_REGISTRY_TOKEN": "synthetic"}), patch.object(RELEASE, "registry_state", return_value=False), patch.object(RELEASE.time, "sleep"), patch.object(RELEASE, "request", return_value=b"{}") as request:
            with self.assertRaises(ValueError):
                RELEASE.publish_crate("0.4.0", "digest", {}, b"archive")
        request.assert_called_once()

    def test_tag_conflicts_refuse_before_signing_or_pushing(self):
        for reference, target in [({"object": {"type": "commit"}}, "a" * 40), ({"object": {"type": "tag"}}, "b" * 40)]:
            with patch.object(RELEASE, "request", return_value=reference), patch.object(RELEASE, "git", side_effect=["", target]) as git, patch.object(RELEASE.subprocess, "run") as verify:
                with self.assertRaises(ValueError):
                    RELEASE.signed_tag("example/panelist", "0.4.0", "a" * 40)
            verify.assert_not_called()
            self.assertFalse(any(call.args[0] == "push" for call in git.call_args_list))

    def test_new_tag_preserves_offline_rekor_bundle_mode_without_force(self):
        with patch.object(RELEASE, "request", return_value=None), patch.object(RELEASE, "git", side_effect=["", "", "a" * 40]) as git, patch.object(RELEASE.subprocess, "run"):
            self.assertEqual(RELEASE.signed_tag("example/panelist", "0.4.0", "a" * 40), ("v0.4.0", False))
        signing = git.call_args_list[1].args
        self.assertIn("gitsign.rekorMode=offline", signing)
        self.assertIn("-s", signing)
        self.assertNotIn("--force", signing)

    def test_signature_verifies_workflow_identity_issuer_and_dispatch_trigger(self):
        with patch.object(RELEASE, "request", return_value={"object": {"type": "tag"}}), patch.object(RELEASE, "git", side_effect=["", "a" * 40]), patch.object(RELEASE.subprocess, "run") as verify:
            self.assertEqual(RELEASE.signed_tag("example/panelist", "0.4.0", "a" * 40), ("v0.4.0", True))
        command = verify.call_args.args[0]
        self.assertIn("https://github.com/example/panelist/.github/workflows/release-plz.yml@refs/heads/main", command)
        self.assertIn("https://token.actions.githubusercontent.com", command)
        self.assertIn("workflow_dispatch", command)

    def test_conflicting_github_receipt_has_no_write_effect(self):
        release = {"id": 1, "tag_name": "v0.4.0", "prerelease": False, "draft": False,
                   "body": "Public notes", "assets": [{"name": "cargo-package-manifest.json", "digest": "sha256:wrong"}]}
        with patch.dict(os.environ, {"RELEASE_NOTES": "Public notes"}), patch.object(RELEASE, "request", return_value=release) as request:
            with self.assertRaises(ValueError):
                RELEASE.publish_github("example/panelist", "v0.4.0", "a" * 40, "b" * 40, "0.4.0", [], "synthetic")
        request.assert_called_once()
        self.assertNotIn("method", request.call_args.kwargs)

    def test_verified_github_receipt_reconciles_without_upload_or_edit(self):
        receipt = {"schema_version": 1, "repository": "example/panelist", "package": "panelist", "version": "0.4.0",
                   "source_sha": "a" * 40, "release_sha": "b" * 40, "index_url": "https://index.crates.io/", "distributions": []}
        digest = "sha256:" + hashlib.sha256(json.dumps(receipt, sort_keys=True).encode()).hexdigest()
        release = {"id": 1, "tag_name": "v0.4.0", "prerelease": False, "draft": False,
                   "body": "Public notes", "assets": [{"name": "cargo-package-manifest.json", "digest": digest}]}
        with patch.dict(os.environ, {"RELEASE_NOTES": "Public notes"}), patch.object(RELEASE, "request", return_value=release) as request:
            RELEASE.publish_github("example/panelist", "v0.4.0", "a" * 40, "b" * 40, "0.4.0", [], "synthetic")
        self.assertEqual(request.call_count, 2)
        self.assertTrue(all("method" not in call.kwargs for call in request.call_args_list))

    def test_redirects_refuse_credential_forwarding(self):
        with self.assertRaises(ValueError):
            RELEASE.NoRedirect().redirect_request(None, None, 302, "redirect", {}, "https://example.test")


if __name__ == "__main__":
    unittest.main()
