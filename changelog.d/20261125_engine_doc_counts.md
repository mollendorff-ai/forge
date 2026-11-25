### Changed

- **The docs no longer say how many unit tests Forge has** (roadmap E-DOC-COUNTS). "over 3,350 tests" went stale with every merge, turned `main` red when another lane's merge crossed a step of 50, and says nothing about quality. `README.md` and `VALIDATION.md` now name what the tests check; the numbers that stay are the ones a reader can verify: the coverage badge, the exact count of Gnumeric cases, and the comparisons with R, Gambit and Excel. `validation/repo` no longer counts tests, and fails if a count comes back.
