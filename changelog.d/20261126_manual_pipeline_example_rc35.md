### Changed

- **The pipeline page's runs are the rc.35 workflow's.** Its "It has run" table links four pull requests of `forge-pipeline-example` on 2026-11-26: the workflow moved to rc.35 and a price change with its results calculated pass, and a price change and a new formula with no results fail, both at "Every stored result is what its formula gives", since `forge validate` now fails a formula with no stored result itself. The earlier table was rc.34's, with a step of its own for that case.
