"""Restore the original DW3 packages and verify all assets before publication."""

import base64
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import urllib.request


ROOT = Path(__file__).resolve().parents[2]
DOCUMENTS = ROOT / "docs/dw3-release"
MANIFEST = json.loads((DOCUMENTS / "RELEASE_MANIFEST.json").read_text())
TRANSFER = json.loads((Path(__file__).parent / "dw3-transfer.json").read_text())


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def verify_file(path, expected):
    require(path.is_file(), f"Missing asset: {path.name}")
    data = path.read_bytes()
    require(len(data) == expected["size_bytes"], f"Size mismatch: {path.name}")
    digest = hashlib.sha256(data).hexdigest()
    require(digest == expected["sha256"], f"SHA-256 mismatch: {path.name}")
    print(f"Verified {path.name}: {digest}", flush=True)


def restore(destination):
    expected = {
        asset["name"]: asset
        for asset in MANIFEST["assets"]
        if asset["kind"] == "original-package"
    }
    require({p["name"] for p in TRANSFER} == set(expected), "Package list mismatch")
    destination.mkdir(parents=True, exist_ok=False)
    for package in TRANSFER:
        name = package["name"]
        require(Path(name).name == name, "Invalid package basename")
        require(package["size"] == expected[name]["size_bytes"], "Package size mismatch")
        require(package["sha256"] == expected[name]["sha256"], "Package digest mismatch")
        with (destination / name).open("wb") as output:
            for chunk in package["chunks"]:
                require(output.tell() == chunk["offset"], "Non-contiguous package chunks")
                url = (
                    "https://api.github.com/repos/"
                    + os.environ["GH_REPO"]
                    + "/git/blobs/"
                    + chunk["git_blob"]
                )
                request = urllib.request.Request(
                    url,
                    headers={
                        "Authorization": "Bearer " + os.environ["GH_TOKEN"],
                        "Accept": "application/vnd.github+json",
                        "X-GitHub-Api-Version": "2022-11-28",
                        "User-Agent": "FilmCraft-DW3-release",
                    },
                )
                with urllib.request.urlopen(request, timeout=120) as response:
                    blob = json.load(response)
                require(blob["sha"] == chunk["git_blob"], "Unexpected blob identifier")
                require(blob["encoding"] == "base64", "Unexpected blob encoding")
                raw = base64.b64decode("".join(blob["content"].split()), validate=True)
                require(len(raw) == chunk["size"] == blob["size"], "Chunk size mismatch")
                header = b"blob " + str(len(raw)).encode("ascii") + b"\0"
                require(
                    hashlib.sha1(header + raw).hexdigest() == chunk["git_blob"],
                    "Chunk content mismatch",
                )
                output.write(raw)
        verify_file(destination / name, expected[name])


def verify_downloads(destination):
    expected = {asset["name"]: asset for asset in MANIFEST["assets"]}
    for name in ("SHA256SUMS.txt", "RELEASE_MANIFEST.json"):
        data = (DOCUMENTS / name).read_bytes()
        expected[name] = {
            "size_bytes": len(data),
            "sha256": hashlib.sha256(data).hexdigest(),
        }
    require(
        {path.name for path in destination.iterdir()} == set(expected),
        "Downloaded asset list differs from the complete release manifest",
    )
    for name, asset in expected.items():
        verify_file(destination / name, asset)
    state = json.loads(
        subprocess.check_output(
            [
                "gh",
                "api",
                "repos/" + os.environ["GH_REPO"] + "/releases/" + os.environ["DW3_RELEASE_ID"],
            ],
            text=True,
        )
    )
    require(state["draft"], "Expected the original release draft")
    require(state["tag_name"] == MANIFEST["tag"], "Release tag mismatch")
    require(state["target_commitish"] == MANIFEST["source_commit"], "Source commit mismatch")
    require({asset["name"] for asset in state["assets"]} == set(expected), "Asset list changed")
    for asset in state["assets"]:
        require(asset["state"] == "uploaded", "Incomplete upload")
        item = expected[asset["name"]]
        require(asset["size"] == item["size_bytes"], "Server asset size mismatch")
        if asset.get("digest"):
            require(asset["digest"] == "sha256:" + item["sha256"], "Server digest mismatch")
    print("All nine release assets match the original verified files.", flush=True)


if __name__ == "__main__":
    require(len(sys.argv) == 3, "Usage: publish_dw3_originals.py restore|verify DIRECTORY")
    if sys.argv[1] == "restore":
        restore(Path(sys.argv[2]))
    elif sys.argv[1] == "verify":
        verify_downloads(Path(sys.argv[2]))
    else:
        raise RuntimeError("Unknown operation")
