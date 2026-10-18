#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Prepare a selected public release without giving source code publish credentials."""

import os
import gzip
import hashlib
import io
import json
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def upload_metadata(document):
    """Cargo's public registry upload schema, from its normalized manifest."""
    package = document["package"]
    metadata = {key.replace("-", "_"): package.get(key) for key in (
        "description", "documentation", "homepage", "license", "license-file",
        "repository", "links", "rust-version", "readme",
    )}
    metadata["readme_file"] = metadata.pop("readme")
    metadata.update(name=package["name"], vers=package["version"],
                    authors=package.get("authors", []), keywords=package.get("keywords", []),
                    categories=package.get("categories", []), features=document.get("features", {}), deps=[])
    groups = [(None, document)] + list(document.get("target", {}).items())
    for target, sections in groups:
        for table, kind in (("dependencies", "normal"), ("dev-dependencies", "dev"), ("build-dependencies", "build")):
            for name, dependency in sections.get(table, {}).items():
                if isinstance(dependency, str):
                    dependency = {"version": dependency}
                if any(key in dependency for key in ("path", "git", "registry", "registry-index")):
                    raise ValueError("Release dependencies must use the default public registry")
                metadata["deps"].append({
                    "name": dependency.get("package", name), "version_req": dependency["version"],
                    "features": dependency.get("features", []), "optional": dependency.get("optional", False),
                    "default_features": dependency.get("default-features", True), "target": target,
                    "kind": kind, "registry": None,
                    "explicit_name_in_toml": name if "package" in dependency else None,
                })
    return metadata


def prepare(root, version, package):
    if os.environ.get("CARGO_TARGET_DIR"):
        raise ValueError("Release bundle requires the checkout target directory")
    if any(os.environ.get(key) for key in ("CARGO_REGISTRY_TOKEN", "GH_TOKEN", "GITHUB_TOKEN")):
        raise ValueError("Package preparation must not receive publishing credentials")
    if (package != "panelist" or not re.fullmatch(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", version)
            or version == "0.0.0" or any(int(part) > 4294967295 for part in version.split("."))):
        raise ValueError("Expected panelist and a nonzero stable release version")
    manifest_path = root / "Cargo.toml"
    lock_path = root / "Cargo.lock"
    manifest = manifest_path.read_text()
    lock = lock_path.read_text()
    workspace = tomllib.loads(manifest)["workspace"]
    if workspace["members"] not in (["crates/panelist"], ["crates/panelist", "crates/panelist-macros"]):
        raise ValueError("Release packaging requires the reviewed Cargo workspace")
    publish = workspace["package"].get("publish", True)
    if publish is not True and publish != ["crates-io"]:
        raise ValueError("Panelist must permit crates.io publication")
    original = workspace["package"]["version"]
    prepared, changes = re.subn(
        r'(\[workspace\.package\]\nversion\s*=\s*)"' + re.escape(original) + '"',
        lambda match: match[1] + json.dumps(version), manifest,
    )
    packages = ["panelist-macros", "panelist"] if "crates/panelist-macros" in workspace["members"] else ["panelist"]
    if len(packages) == 2:
        prepared, dependency_changes = re.subn(
            r'(panelist-macros = \{ version = )"=' + re.escape(original) + '"',
            lambda match: match[1] + json.dumps("=" + version), prepared,
        )
        if dependency_changes != 1:
            raise ValueError("Macro dependency must pin the lockstep release exactly")
    prepared_lock, lock_changes = re.subn(
        r'(\[\[package\]\]\nname = "(?:panelist|panelist-macros)"\nversion = )"' + re.escape(original) + '"',
        lambda match: match[1] + json.dumps(version), lock,
    )
    if changes != 1 or lock_changes != len(packages):
        raise ValueError("Workspace and locked package versions must match exactly")
    # Validate both prepared documents before either write. Registry dependencies
    # retain their original locked versions and checksums.
    tomllib.loads(prepared)
    tomllib.loads(prepared_lock)
    manifest_path.write_text(prepared)
    lock_path.write_text(prepared_lock)
    subprocess.run([
        "cargo", "package", "--locked", "--allow-dirty",
        "--workspace", "--registry", "crates-io",
    ], cwd=root, check=True)
    destination = root / "target/release-bundle"
    if destination.is_symlink():
        raise ValueError("Release bundle path must not be a symlink")
    if destination.exists():
        shutil.rmtree(destination)
    destination.mkdir(parents=True)
    proofs = []
    uploads = {}
    for name in packages:
        archive_bytes = (root / "target/package" / f"{name}-{version}.crate").read_bytes()
        filename = f"{name}-{version}.crate"
        (destination / filename).write_bytes(archive_bytes)
        proofs.append({"package": name, "filename": filename, "sha256": hashlib.sha256(archive_bytes).hexdigest()})
        with tarfile.open(fileobj=io.BytesIO(archive_bytes), mode="r:gz") as crate:
            prefix = f"{name}-{version}/"
            document = tomllib.loads(crate.extractfile(prefix + "Cargo.toml").read().decode())
            metadata = upload_metadata(document)
            readme = document["package"].get("readme")
            metadata["readme"] = crate.extractfile(prefix + readme).read().decode() if readme else None
            uploads[name] = metadata
    (destination / "upload.json").write_text(json.dumps(uploads, sort_keys=True) + "\n")
    source = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
    (destination / "provenance.json").write_text(json.dumps({
        "format": 1, "package": package, "version": version, "source_sha": source,
        "distributions": proofs,
    }, sort_keys=True) + "\n")
    # Only these versioned metadata files may enter the release commit.
    with (destination / "metadata.tar.gz").open("wb") as output:
        with gzip.GzipFile(fileobj=output, mode="wb", filename="", mtime=0) as compressed:
            with tarfile.open(fileobj=compressed, mode="w") as metadata:
                for name in ("Cargo.lock", "Cargo.toml"):
                    data = (root / name).read_bytes()
                    header = tarfile.TarInfo(name)
                    header.size = len(data)
                    header.mode = 0o644
                    metadata.addfile(header, io.BytesIO(data))


def main():
    source = os.environ["SOURCE_SHA"]
    if (not re.fullmatch(r"[0-9a-f]{40}", source)
            or subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip() != source):
        raise ValueError("Checkout does not match the selected source")
    if os.environ["SKIP_RELEASE_VALIDATION"] not in ("true", "false"):
        raise ValueError("Invalid validation selection")
    if os.environ["SKIP_RELEASE_VALIDATION"] == "false":
        subprocess.run(["make", "ci"], cwd=ROOT, check=True)
    prepare(ROOT, os.environ["REQUESTED_VERSION"], "panelist")


if __name__ == "__main__":
    main()
