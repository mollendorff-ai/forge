### Changed

- **CI measures the coverage of the repository checks (`validation/repo`) and fails below a floor.** The checks of the docs, examples and rules are code too; they now run under `cargo llvm-cov` in the Repository checks job, which names each file with a miss and fails when lines, functions or regions fall below their floor (98.9%, 98.6% and 98.6%, a tenth below today's). The floors only rise, to 100%. No behaviour of Forge changes.
