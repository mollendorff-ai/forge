### Fixed

- **`forge upgrade` raises a 5.3.0, 5.4.0 or 5.5.0 file, games and dates included, by changing its version line only.** A 5.5.0 file with `games:`, or a 5.4.0 file with a date as a scalar, was refused with "'c' is a plain number, which is the short form of model format 5.1.0", after a warning that its comments could not be kept. Every format from 5.1.0 on is already in the short form, so its comments and layout are now kept as they are.
