### Changed

- **The VS Code extension's check and the Mermaid check install their npm packages from a committed lockfile.** `editors/vscode` and `validation/mermaid` had no `package-lock.json` (both were ignored), so CI's `npm install` took the newest version of every dependency of their dependencies on the day it ran. Both now commit one and CI runs `npm ci`, which is what Scorecard's Pinned-Dependencies check asks of a build. No behaviour of Forge changes.
