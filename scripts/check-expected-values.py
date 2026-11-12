#!/usr/bin/env python3
"""Fail when a pull request changes or removes an expected value of the
validation suites without saying so in its title (ADR-057, rule 5).

An expected value is anything under a key whose name contains `expected`
(`expected:`, `r_expected:` with the prices and posteriors in it) or under a
`tolerance` key, in the spec files of validation/*/tests. Adding cases is
free; a value that is different, or gone, after the pull request is a change.

Usage: check-expected-values.py <base commit> <pull request title>
"""

import subprocess
import sys

import yaml

PHRASE = "changes expected values"


def specs_at(commit):
    """The spec files of the suites at `commit`, as {path: text}."""
    listed = subprocess.run(
        ["git", "ls-tree", "-r", "--name-only", commit, "--", "validation"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.split()
    files = {}
    for path in listed:
        if "/tests/" in path and path.endswith((".yaml", ".yml")):
            files[path] = subprocess.run(
                ["git", "show", f"{commit}:{path}"],
                check=True,
                capture_output=True,
                text=True,
            ).stdout
    return files


def is_expected_key(key):
    key = str(key).lower()
    return "expected" in key or key == "tolerance" or key.endswith("_tolerance")


def expected_values(node, path=(), inside=False):
    """Every expected value in a spec, as {path: value}."""
    found = {}
    if isinstance(node, dict):
        for key, value in node.items():
            here = path + (str(key),)
            found.update(expected_values(value, here, inside or is_expected_key(key)))
    elif isinstance(node, list):
        for index, value in enumerate(node):
            found.update(expected_values(value, path + (str(index),), inside))
    elif inside:
        found[path] = node
    return found


def main():
    base, title = sys.argv[1], sys.argv[2]
    head = {path: text for path, text in specs_at("HEAD").items()}
    changed = []
    for path, text in specs_at(base).items():
        before = expected_values(yaml.safe_load(text) or {})
        after = expected_values(yaml.safe_load(head.get(path, "")) or {}) if path in head else {}
        for key, value in before.items():
            if key not in after:
                changed.append(f"{path}: {'.'.join(key)} = {value!r} is gone")
            elif after[key] != value:
                changed.append(f"{path}: {'.'.join(key)} {value!r} -> {after[key]!r}")
    if not changed:
        print("No expected value of the validation suites changed.")
        return 0
    print(f"{len(changed)} expected value(s) of the validation suites changed or went:")
    for line in changed[:50]:
        print(f"  {line}")
    if len(changed) > 50:
        print(f"  ... and {len(changed) - 50} more")
    if PHRASE in title.lower():
        print(f'The title says "{PHRASE}"; the description must say where each new value comes from.')
        return 0
    print(
        f'\nThe title does not say "{PHRASE}". Add it, and say in the description where each '
        "new value comes from (Excel, Gnumeric, R; the version and the date). ADR-057, rule 5."
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
