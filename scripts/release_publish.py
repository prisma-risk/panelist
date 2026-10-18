#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Reconcile a verified crate, signed tag and public GitHub release."""

import base64
import hashlib
import io
import json
import os
from pathlib import Path
import re
import struct
import subprocess
import sys
import tarfile
import time
import tomllib
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
BUNDLE = ROOT / "target/release-bundle"
MAXIMUM = 32 * 1024 * 1024


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, message, headers, new_url):
        raise ValueError("Release endpoints must not redirect")


def request(url, method="GET", payload=None, token=None, binary=False, missing=False):
    headers = {"User-Agent": "panelist-release/1", "Accept": "application/octet-stream" if binary else "application/json"}
    if token:
        headers["Authorization"] = token if url == "https://crates.io/api/v1/crates/new" else f"Bearer {token}"
    if payload is not None:
        headers["Content-Type"] = "application/octet-stream" if binary else "application/json"
        if not binary:
            payload = json.dumps(payload).encode()
    try:
        with urllib.request.build_opener(NoRedirect).open(
                urllib.request.Request(url, data=payload, headers=headers, method=method), timeout=30) as response:
            content = response.read(MAXIMUM + 1)
            if len(content) > MAXIMUM:
                raise ValueError("Release response exceeds its size limit")
            return content if binary else json.loads(content)
    except urllib.error.HTTPError as error:
        if error.code == 404 and missing:
            return None
        raise ValueError(f"Release endpoint refused {method} (HTTP {error.code})") from None
    except (urllib.error.URLError, TimeoutError):
        raise ValueError("Release endpoint unavailable; reconcile before retrying") from None


def git(*arguments, env=None):
    # Source hooks and ambient signing settings cannot execute during metadata preparation.
    environment = {key: value for key, value in os.environ.items() if key not in (
        "CARGO_REGISTRY_TOKEN", "GH_TOKEN", "GITHUB_TOKEN")}
    if env:
        environment.update(env)
    return subprocess.check_output(["git", "-c", "core.hooksPath=/dev/null", *arguments],
                                   cwd=ROOT, env=environment, text=True).strip()


def load_bundle():
    source = os.environ["SOURCE_SHA"]
    version = os.environ["REQUESTED_VERSION"]
    repository = os.environ["GITHUB_REPOSITORY"]
    if (not re.fullmatch(r"[0-9a-f]{40}", source)
            or not re.fullmatch(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", version)
            or version == "0.0.0" or any(int(p) > 4294967295 for p in version.split("."))
            or not re.fullmatch(r"[A-Za-z0-9_-]+/[A-Za-z0-9_.-]+", repository)
            or os.environ["REQUEST_ID"] != "release-" + source[:32]
            or os.environ["ORIGIN"] != "release"
            or not 0 < len(os.environ["RELEASE_NOTES"].encode()) <= 12000
            or git("rev-parse", "HEAD") != source):
        raise ValueError("Invalid exact-source release request")
    git("merge-base", "--is-ancestor", source, "origin/main")
    proof = json.loads((BUNDLE / "provenance.json").read_bytes())
    distributions = proof.get("distributions", [])
    names = [entry["package"] for entry in distributions]
    if names not in (["panelist"], ["panelist-macros", "panelist"]):
        raise ValueError("Unexpected lockstep package set or publication order")
    expected = {"metadata.tar.gz", "provenance.json", "upload.json"} | {f"{name}-{version}.crate" for name in names}
    if {path.name for path in BUNDLE.iterdir()} != expected:
        raise ValueError("Unexpected release bundle contents")
    if any(path.is_symlink() or not path.is_file() or path.stat().st_size > MAXIMUM for path in BUNDLE.iterdir()):
        raise ValueError("Invalid release bundle file")
    archives = {}
    for entry in distributions:
        name = entry["package"]
        filename = f"{name}-{version}.crate"
        archive = (BUNDLE / filename).read_bytes()
        digest = hashlib.sha256(archive).hexdigest()
        if entry != {"package": name, "filename": filename, "sha256": digest}:
            raise ValueError("Release archive provenance mismatch")
        archives[name] = archive
    if {key: value for key, value in proof.items() if key != "distributions"} != {
            "format": 1, "package": "panelist", "version": version, "source_sha": source}:
        raise ValueError("Release archive source mismatch")
    with tarfile.open(BUNDLE / "metadata.tar.gz") as metadata:
        members = metadata.getmembers()
        if sorted(m.name for m in members) != ["Cargo.lock", "Cargo.toml"] or any(not m.isfile() or m.size > 1024 * 1024 for m in members):
            raise ValueError("Unexpected release commit metadata")
        prepared = {m.name: metadata.extractfile(m).read() for m in members}
    workspace = tomllib.loads(prepared["Cargo.toml"].decode())["workspace"]
    locked = tomllib.loads(prepared["Cargo.lock"].decode())["package"]
    if (workspace["package"]["version"] != version
            or sorted(workspace["members"]) != sorted("crates/" + name for name in names)
            or sorted((p["name"], p["version"]) for p in locked if p["name"] in names) != sorted((name, version) for name in names)):
        raise ValueError("Prepared version mismatch")
    uploads = json.loads((BUNDLE / "upload.json").read_bytes())
    if set(uploads) != set(names):
        raise ValueError("Upload metadata package set mismatch")
    # Bind each API document to the normalized manifest inside its verified archive.
    from release_package import upload_metadata
    for name, archive in archives.items():
        with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as crate:
            prefix = f"{name}-{version}/"
            member = crate.getmember(prefix + "Cargo.toml")
            if not member.isfile() or member.size > 1024 * 1024:
                raise ValueError("Invalid crate manifest")
            document = tomllib.loads(crate.extractfile(member).read().decode())
            vcs = json.loads(crate.extractfile(prefix + ".cargo_vcs_info.json").read())
            if vcs["git"]["sha1"] != source:
                raise ValueError("Crate archive belongs to a different source commit")
            actual = upload_metadata(document)
            readme = document["package"].get("readme")
            actual["readme"] = crate.extractfile(prefix + readme).read().decode() if readme else None
        if actual["name"] != name or actual["vers"] != version or uploads[name] != actual:
            raise ValueError("Upload metadata differs from the verified archive")
    return source, version, repository, archives, distributions, prepared, uploads


def registry_state(version, digest, package="panelist"):
    index = request(f"https://index.crates.io/{package[:2]}/{package[2:4]}/{package}", binary=True, missing=True)
    if index is None:
        return False
    found = []
    for line in index.splitlines():
        row = json.loads(line)
        if row["name"] != package:
            raise ValueError("Unexpected registry package")
        if row["vers"] == version:
            found.append(row)
    if not found:
        return False
    if len(found) != 1 or found[0]["yanked"] is not False or found[0]["cksum"] != digest:
        raise ValueError("Published version is occupied by different or yanked content")
    archive = request(f"https://static.crates.io/crates/{package}/{package}-{version}.crate", binary=True)
    if hashlib.sha256(archive).hexdigest() != digest:
        raise ValueError("Published archive checksum mismatch")
    return True


def publish_crate(version, digest, metadata, archive):
    package = metadata.get("name", "panelist")
    if registry_state(version, digest, package):
        return
    document = json.dumps(metadata, sort_keys=True).encode()
    payload = struct.pack("<I", len(document)) + document + struct.pack("<I", len(archive)) + archive
    # One write per attempt. A lost response is reconciled by independent reads.
    failure = None
    try:
        result = request("https://crates.io/api/v1/crates/new", method="PUT", payload=payload,
                         token=os.environ["CARGO_REGISTRY_TOKEN"], binary=True)
        if json.loads(result).get("errors"):
            failure = ValueError("Registry rejected the publication")
    except ValueError as error:
        failure = error
    for attempt in range(12):
        if registry_state(version, digest, package):
            return
        if attempt != 11:
            time.sleep(5)
    raise ValueError("Publication remains unverified; reconcile before retrying") from failure


def release_commit(source, version, prepared):
    # Reconstruct deterministically so reruns never move an existing tag.
    timestamp = git("show", "-s", "--format=%ct", source)
    environment = {"GIT_AUTHOR_NAME": "Release", "GIT_AUTHOR_EMAIL": "release@users.noreply.github.com",
                   "GIT_COMMITTER_NAME": "Release", "GIT_COMMITTER_EMAIL": "release@users.noreply.github.com",
                   "GIT_AUTHOR_DATE": f"@{timestamp} +0000", "GIT_COMMITTER_DATE": f"@{timestamp} +0000",
                   "GIT_INDEX_FILE": str(BUNDLE / "release.index")}
    try:
        git("read-tree", source, env=environment)
        for name, content in prepared.items():
            path = BUNDLE / (name + ".prepared")
            path.write_bytes(content)
            blob = git("hash-object", "-w", str(path))
            git("update-index", "--add", "--cacheinfo", f"100644,{blob},{name}", env=environment)
        tree = git("write-tree", env=environment)
        return git("-c", "commit.gpgsign=false", "commit-tree", tree, "-p", source, "-m",
                   f"chore(release): v{version}\n\nKargo-Source-SHA: {source}", env=environment)
    finally:
        (BUNDLE / "release.index").unlink(missing_ok=True)
        for name in prepared:
            (BUNDLE / (name + ".prepared")).unlink(missing_ok=True)


def signed_tag(repository, version, commit):
    tag = "v" + version
    reference = request(f"https://api.github.com/repos/{repository}/git/ref/tags/{tag}", missing=True)
    if reference is not None:
        if reference["object"]["type"] != "tag":
            raise ValueError("Existing tag is not signed and annotated")
        git("fetch", "origin", f"refs/tags/{tag}:refs/tags/{tag}")
    elif not git("tag", "--list", tag):
        git("-c", "gpg.format=x509", "-c", "gpg.x509.program=gitsign", "-c", "gitsign.rekorMode=offline", "tag", "-s", tag, commit,
            "-m", f"Release {tag}", env={"GIT_COMMITTER_NAME": "Release", "GIT_COMMITTER_EMAIL": "release@users.noreply.github.com"})
    if git("rev-parse", f"{tag}^{{}}") != commit:
        raise ValueError("Existing tag targets a different release commit")
    environment = {k: v for k, v in os.environ.items() if k not in ("CARGO_REGISTRY_TOKEN", "GH_TOKEN", "GITHUB_TOKEN")}
    subprocess.run(["gitsign", "verify-tag", tag, "--certificate-identity",
                    f"https://github.com/{repository}/.github/workflows/release-plz.yml@refs/heads/main",
                    "--certificate-oidc-issuer", "https://token.actions.githubusercontent.com",
                    "--certificate-github-workflow-trigger", "workflow_dispatch"], cwd=ROOT, env=environment, check=True)
    return tag, reference is not None


def publish_github(repository, tag, source, commit, version, distributions, token):
    endpoint = f"https://api.github.com/repos/{repository}/releases"
    receipt = {"schema_version": 1, "repository": repository, "package": "panelist", "version": version,
               "source_sha": source, "release_sha": commit, "index_url": "https://index.crates.io/",
               "distributions": sorted(({"filename": entry["filename"], "sha256": entry["sha256"]} for entry in distributions), key=lambda entry: entry["filename"])}
    release = request(endpoint + f"/tags/{tag}", token=token, missing=True)
    if release is None:
        release = request(endpoint, method="POST", token=token, payload={
            "tag_name": tag, "name": tag, "body": os.environ["RELEASE_NOTES"], "draft": True, "prerelease": False})
    if release["tag_name"] != tag or release["prerelease"] or release.get("body") != os.environ["RELEASE_NOTES"]:
        raise ValueError("Conflicting GitHub release")
    assets = [a for a in release["assets"] if a["name"] == "cargo-package-manifest.json"]
    if len(assets) > 1:
        raise ValueError("Duplicate publication receipt")
    expected_digest = "sha256:" + hashlib.sha256(json.dumps(receipt, sort_keys=True).encode()).hexdigest()
    if assets:
        # GitHub records a SHA-256 for uploaded assets, avoiding credentialed
        # redirects to a download host during partial-release reconciliation.
        if assets[0].get("digest") != expected_digest:
            raise ValueError("Conflicting or unavailable publication receipt digest")
    else:
        url = f"https://uploads.github.com/repos/{repository}/releases/{release['id']}/assets?name=cargo-package-manifest.json"
        uploaded = json.loads(request(url, method="POST", token=token, payload=json.dumps(receipt, sort_keys=True).encode(), binary=True))
        if uploaded.get("digest") != expected_digest:
            raise ValueError("Uploaded receipt digest unavailable or mismatched")
    if release["draft"]:
        request(endpoint + f"/{release['id']}", method="PATCH", token=token, payload={"draft": False})
    published = request(endpoint + f"/tags/{tag}", token=token)
    if published["id"] != release["id"] or published["draft"] or published["prerelease"] or published["tag_name"] != tag:
        raise ValueError("GitHub release publication remains unverified")


def main():
    source, version, repository, archives, distributions, prepared, uploads = load_bundle()
    if sys.argv[1:] == ["--check"]:
        print("Verified exact-source release bundle")
        return
    if sys.argv[1:]:
        raise ValueError("Unexpected release arguments")
    commit = release_commit(source, version, prepared)
    tag, pushed = signed_tag(repository, version, commit)
    for entry in distributions:
        name = entry["package"]
        publish_crate(version, entry["sha256"], uploads[name], archives[name])
    token = os.environ["GH_TOKEN"]
    if not pushed:
        authorization = base64.b64encode(f"x-access-token:{token}".encode()).decode()
        git("push", f"https://github.com/{repository}.git", f"refs/tags/{tag}", env={
            "GIT_CONFIG_COUNT": "1", "GIT_CONFIG_KEY_0": "http.https://github.com/.extraheader",
            "GIT_CONFIG_VALUE_0": "AUTHORIZATION: basic " + authorization})
    reference = request(f"https://api.github.com/repos/{repository}/git/ref/tags/{tag}")
    if reference["object"]["type"] != "tag" or reference["object"]["sha"] != git("rev-parse", tag):
        raise ValueError("Remote signed tag identity mismatch")
    publish_github(repository, tag, source, commit, version, distributions, token)
    print(f"Verified crates.io publication and signed {tag}")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError):
        # Never print credential-bearing request or process details.
        sys.exit("Release reconciliation failed; inspect publication state before retrying")
