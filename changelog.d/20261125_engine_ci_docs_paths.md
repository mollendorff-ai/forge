### Fixed

- **A pull request that changes only pages runs the repository checks** (roadmap E-CI-DOCS-PATHS). CI's path filter had no `docs/**`, `**/*.md` or `skills/**`, so a pull request of pages ran only the Mermaid check; the checks of the docs now run on it, and a test in `validation/repo` holds the filter.
