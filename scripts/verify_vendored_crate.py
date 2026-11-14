#!/usr/bin/env python3
"""Prove that vendor/rusqlite_migration is the crates.io release plus one patch.

The published archive is checked against the crates.io index checksum, read
in memory (nothing is extracted or executed), the recorded patch is applied to
it, and the result must equal the vendored tree file for file, byte for byte.

    python3 scripts/verify_vendored_crate.py            # downloads the archive
    python3 scripts/verify_vendored_crate.py FILE.crate # uses a local copy

Needs only python3 (and network access unless FILE.crate is given).
"""

import hashlib
import io
import sys
import tarfile
import urllib.request
from pathlib import Path

NAME = "rusqlite_migration"
VERSION = "2.7.0"
# `cksum` of rusqlite_migration 2.7.0 in the crates.io index.
SHA256 = "cb11df4f58eb4c6d5506643d678899d7258b40ba85ac5eee2ec9b4a7868b9787"
URL = f"https://static.crates.io/crates/{NAME}/{NAME}-{VERSION}.crate"
VENDOR_DIR = Path(__file__).resolve().parent.parent / "vendor" / NAME

# The only change: accept rusqlite 0.42, whose bundled SQLCipher ships
# SQLite 3.54.1 (RUSTSEC-2026-0459). 2.7.0 compiles unchanged against it.
PATCHES = {
    "Cargo.toml": (
        b'[dependencies.rusqlite]\nversion = "0.41.0"',
        b'[dependencies.rusqlite]\nversion = "0.42.0"',
    ),
}


def upstream_files(archive: bytes) -> dict[str, bytes]:
    prefix = f"{NAME}-{VERSION}/"
    files = {}
    with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as tar:
        for member in tar.getmembers():
            if not member.isfile():
                continue
            if not member.name.startswith(prefix):
                raise SystemExit(f"unexpected archive path: {member.name}")
            files[member.name[len(prefix) :]] = tar.extractfile(member).read()
    return files


def main() -> int:
    if len(sys.argv) > 1:
        archive = Path(sys.argv[1]).read_bytes()
    else:
        with urllib.request.urlopen(URL, timeout=60) as response:
            archive = response.read()
    digest = hashlib.sha256(archive).hexdigest()
    if digest != SHA256:
        print(f"checksum mismatch: {digest} != {SHA256}", file=sys.stderr)
        return 1

    expected = upstream_files(archive)
    for path, (old, new) in PATCHES.items():
        if expected[path].count(old) != 1:
            print(f"patch context not found exactly once in {path}", file=sys.stderr)
            return 1
        expected[path] = expected[path].replace(old, new)

    actual = {
        str(p.relative_to(VENDOR_DIR)): p.read_bytes()
        for p in VENDOR_DIR.rglob("*")
        if p.is_file()
    }
    problems = [f"missing: {p}" for p in sorted(expected.keys() - actual.keys())]
    problems += [f"unexpected: {p}" for p in sorted(actual.keys() - expected.keys())]
    problems += [
        f"differs: {p}"
        for p in sorted(expected.keys() & actual.keys())
        if expected[p] != actual[p]
    ]
    if problems:
        print("\n".join(problems), file=sys.stderr)
        return 1
    print(
        f"vendor/{NAME} = {NAME} {VERSION} (sha256 {SHA256[:12]}…) "
        f"+ {len(PATCHES)} recorded patch; {len(actual)} files identical."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
