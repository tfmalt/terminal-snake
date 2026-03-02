#!/usr/bin/env python3

"""Update terminal-snake Homebrew formula release tag and checksums.

This script fetches release metadata from GitHub and updates one or more
formula files in place.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import subprocess
import sys


REPO = "tfmalt/terminal-snake"
TARGETS = [
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "aarch64-unknown-linux-gnu",
    "x86_64-unknown-linux-gnu",
]


def run_gh_release_view(tag: str | None) -> dict:
    cmd = ["gh", "release", "view", "--repo", REPO, "--json", "tagName,assets"]
    if tag is not None:
        cmd.insert(3, tag)

    try:
        proc = subprocess.run(cmd, check=True, capture_output=True, text=True)
    except FileNotFoundError as exc:
        raise RuntimeError("`gh` is required but was not found in PATH") from exc
    except subprocess.CalledProcessError as exc:
        stderr = exc.stderr.strip() if exc.stderr else str(exc)
        raise RuntimeError(f"failed to query release metadata: {stderr}") from exc

    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        raise RuntimeError("failed to parse JSON from `gh release view`") from exc


def checksum_map(metadata: dict) -> tuple[str, dict[str, str]]:
    tag = metadata["tagName"]
    assets = metadata["assets"]

    shas: dict[str, str] = {}
    for target in TARGETS:
        asset_name = f"terminal-snake-{tag}-{target}.tar.gz"
        asset = next((item for item in assets if item.get("name") == asset_name), None)
        if asset is None:
            raise RuntimeError(f"release asset not found: {asset_name}")

        digest = asset.get("digest")
        if not isinstance(digest, str) or not digest.startswith("sha256:"):
            raise RuntimeError(f"missing sha256 digest for asset: {asset_name}")

        shas[target] = digest.split(":", 1)[1]

    return tag, shas


def update_formula(path: pathlib.Path, tag: str, shas: dict[str, str]) -> bool:
    text = path.read_text(encoding="utf-8")
    original = text

    release_line = f'  RELEASE = "{tag}"'
    if re.search(r'^\s*RELEASE\s*=\s*"[^"]+"\s*$', text, flags=re.MULTILINE):
        text = re.sub(
            r'^\s*RELEASE\s*=\s*"[^"]+"\s*$',
            release_line,
            text,
            count=1,
            flags=re.MULTILINE,
        )
    else:
        text = re.sub(
            r'^(\s*homepage\s+"[^"]+"\n)',
            r"\1" + release_line + "\n",
            text,
            count=1,
            flags=re.MULTILINE,
        )

    for target, sha in shas.items():
        pattern = re.compile(
            rf'(url\s+"[^"\n]*-{re.escape(target)}\.tar\.gz"\n\s*sha256\s+")[0-9a-f]{{64}}(")',
            flags=re.MULTILINE,
        )
        text, count = pattern.subn(rf"\g<1>{sha}\g<2>", text, count=1)
        if count != 1:
            raise RuntimeError(f"could not update sha256 for target {target} in {path}")

    if text != original:
        path.write_text(text, encoding="utf-8")
        return True
    return False


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Update terminal-snake Homebrew formula release and checksums."
    )
    parser.add_argument(
        "--tag",
        help="Release tag to use (for example v0.8.16). Defaults to latest release.",
    )
    parser.add_argument(
        "--formula",
        action="append",
        default=None,
        help=(
            "Formula path to update. Can be passed multiple times. "
            "Default: packaging/homebrew/terminal-snake.rb"
        ),
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    formula_paths = args.formula or ["packaging/homebrew/terminal-snake.rb"]

    metadata = run_gh_release_view(args.tag)
    tag, shas = checksum_map(metadata)

    for formula in formula_paths:
        path = pathlib.Path(formula).expanduser().resolve()
        if not path.exists():
            raise RuntimeError(f"formula file not found: {path}")

        changed = update_formula(path, tag, shas)
        state = "updated" if changed else "unchanged"
        print(f"{state}: {path}")

    print(f"release tag: {tag}")
    for target in TARGETS:
        print(f"{target}: {shas[target]}")

    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except RuntimeError as err:
        print(f"error: {err}", file=sys.stderr)
        raise SystemExit(1)
