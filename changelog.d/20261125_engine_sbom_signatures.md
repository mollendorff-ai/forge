### Added

- **Each release carries an SBOM and a signature for every archive.** `forge.cdx.json` (CycloneDX, made from `Cargo.lock` by `cargo-cyclonedx` 0.5.10) lists what is inside the binaries. Each archive and the SBOM have a keyless Sigstore bundle beside them (`<file>.sigstore`); check one with `cosign verify-blob --bundle <file>.sigstore --certificate-identity-regexp 'https://github.com/mollendorff-ai/forge/' --certificate-oidc-issuer https://token.actions.githubusercontent.com <file>`.
