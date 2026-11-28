### Fixed

- **The repository checks stay covered 100% when `changelog.d/` is empty.** The check of each changelog fragment ran only while there were fragments, so the release of rc.37, which empties the folder, left it uncovered and failed CI's floor. It now takes the folder's files as input and is also tested on made-up ones.
