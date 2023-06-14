#!/usr/bin/env python3

import argparse
import json
import os.path
import re
import shutil
import subprocess
import tempfile
import typing


class Repo:
    path: str
    version_re = re.compile("[^0-9]")

    def __init__(self, path: str):
        self.path = path

    def checkout(self, commit: str):
        subprocess.run(("git", "checkout", "-q", commit), cwd=self.repo_path())

    def clone(self, repo: str):
        shutil.rmtree(self.repo_path(), ignore_errors=True)
        subprocess.run(("git", "clone", "-nq", repo, self.repo_path()))

    def migrations(self) -> typing.Generator[str, None, None]:
        for path in sorted(os.listdir(os.path.join(self.repo_path(), "migrations"))):
            # Diesel versions are essentially any number before the first
            # underscore, with any other characters being ignored.
            yield __class__.version_re.sub(
                "", os.path.basename(path).split("_", maxsplit=1)[0]
            )

    def repo_path(self) -> str:
        return os.path.join(self.path, "checkout")


def current_commit(metadata: str) -> str:
    with open(metadata, "r") as file:
        data = json.load(file)
        return data["crates_io_commit"]


def main(metadata: str, upstream: str):
    with tempfile.TemporaryDirectory() as path:
        # Clone the current crates.io repo into a fresh checkout at the exact
        # commit specified by the metadata so we can enumerate the migrations
        # within.
        #
        # This obviously relies on knowing where this script lives within the
        # repo.
        repo = Repo(path)
        repo.clone(upstream)
        repo.checkout(current_commit(metadata))

        # We just dump out straight SQL so that we don't have to worry about
        # interacting with PostgreSQL from Python — this way, we only need any
        # recent-ish Python 3 version and its standard library. Easy enough to
        # pipe into psql from there.
        print("BEGIN;")
        for version in repo.migrations():
            # This should really do some sort of SQL escaping here, but since
            # the versions _should_ only ever be numeric strings, this is safe
            # enough in practice.
            #
            # The ON CONFLICT clause is there because a fresh database will
            # probably have a migration record for the root migration. We can
            # just ignore that. This also makes the script idempotent.
            print(
                f"INSERT INTO __diesel_schema_migrations (version, run_on) VALUES ('{version}', NOW()) ON CONFLICT DO NOTHING;"
            )
        print("COMMIT;")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Generates a PostgreSQL script that updates the Diesel migrations from the given dump metadata."
    )
    parser.add_argument(
        "-m",
        "--metadata",
        help="path to the database dump metadata.json",
        required=True,
    )
    parser.add_argument(
        "-r",
        "--repo",
        default="https://github.com/rust-lang/crates.io",
        help="repo to clone when enumerating migration versions",
    )
    args = parser.parse_args()

    main(args.metadata, args.repo)
