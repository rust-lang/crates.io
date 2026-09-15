#!/usr/bin/env python3
import argparse
import csv
import json
import sys
import urllib.request
from pathlib import Path

USER_AGENT = "crate-backup (https://github.com/rust-lang/crates.io)"
DOWNLOADS_CSV = "downloads.csv"
REVERSE_DEPENDENCIES_CSV = "reverse-dependencies.csv"
PER_PAGE = 100
DOWNLOADS_CSV_HEADER = ["crate", "version", "downloads"]
REVERSE_DEPENDENCIES_CSV_HEADER = [
    "crate",
    "dependent_crate",
    "dependent_version",
    "requirement",
    "kind",
    "target",
    "optional",
    "default_features",
    "features",
    "downloads",
]


def fetch(url):
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    return urllib.request.urlopen(req)


def list_versions(crate):
    url = f"https://crates.io/api/v1/crates/{crate}/versions"
    with fetch(url) as resp:
        data = json.load(resp)
    return [(v["num"], v["downloads"]) for v in data["versions"]]


def list_reverse_dependencies(crate):
    """Return all available reverse dependencies for a crate."""
    reverse_dependencies = []
    page = 1

    while True:
        url = (
            f"https://crates.io/api/v1/crates/{crate}/reverse_dependencies"
            f"?page={page}&per_page={PER_PAGE}"
        )
        with fetch(url) as resp:
            data = json.load(resp)

        versions = {version["id"]: version for version in data["versions"]}
        dependencies = data["dependencies"]
        reverse_dependencies.extend(
            (dependency, versions[dependency["version_id"]])
            for dependency in dependencies
        )

        if len(reverse_dependencies) >= data["meta"]["total"]:
            break
        page += 1

    return reverse_dependencies


def download_version(crate, version):
    url = f"https://static.crates.io/crates/{crate}/{crate}-{version}.crate"
    dest = Path.cwd() / f"{crate}-{version}.crate"
    if dest.exists():
        print(f"skip {dest.name} (already exists)")
        return
    print(f"download {dest.name}")
    with fetch(url) as resp, open(dest, "wb") as f:
        while chunk := resp.read(64 * 1024):
            f.write(chunk)


def backup_versions(crate, writer):
    """Download all versions of a crate and record their download counts."""
    versions = list_versions(crate)
    print(f"found {len(versions)} versions of {crate}")
    for version, downloads in versions:
        writer.writerow([crate, version, downloads])
        download_version(crate, version)


def record_reverse_dependencies(crate, writer):
    """Record all available reverse dependencies of a crate."""
    reverse_dependencies = list_reverse_dependencies(crate)
    print(f"found {len(reverse_dependencies)} reverse dependencies of {crate}")
    for dependency, version in reverse_dependencies:
        writer.writerow(
            [
                crate,
                version["crate"],
                version["num"],
                dependency["req"],
                dependency["kind"],
                dependency["target"],
                dependency["optional"],
                dependency["default_features"],
                json.dumps(dependency["features"]),
                dependency["downloads"],
            ]
        )


def main():
    parser = argparse.ArgumentParser(
        description="Download all versions and record reverse dependencies of one or more crates."
    )
    parser.add_argument("crates", nargs="+", metavar="CRATE", help="Name of a crate")
    args = parser.parse_args()

    downloads_csv_path = Path.cwd() / DOWNLOADS_CSV
    reverse_dependencies_csv_path = Path.cwd() / REVERSE_DEPENDENCIES_CSV
    with (
        open(downloads_csv_path, "w", newline="") as downloads_file,
        open(reverse_dependencies_csv_path, "w", newline="") as reverse_dependencies_file,
    ):
        downloads_writer = csv.writer(downloads_file)
        downloads_writer.writerow(DOWNLOADS_CSV_HEADER)

        reverse_dependencies_writer = csv.writer(reverse_dependencies_file)
        reverse_dependencies_writer.writerow(REVERSE_DEPENDENCIES_CSV_HEADER)

        for crate in args.crates:
            backup_versions(crate, downloads_writer)
            record_reverse_dependencies(crate, reverse_dependencies_writer)

    print(f"wrote download counts to {downloads_csv_path.name}")
    print(f"wrote reverse dependencies to {reverse_dependencies_csv_path.name}")


if __name__ == "__main__":
    try:
        main()
    except urllib.error.HTTPError as e:
        print(f"HTTP error: {e}", file=sys.stderr)
        sys.exit(1)
