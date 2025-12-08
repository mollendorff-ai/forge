# Forge Hosting Architecture

**Last Updated:** December 2025

## Overview

Forge uses a hybrid hosting architecture designed for maximum security and control over proprietary source code while maintaining public visibility for marketing and enterprise leads.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    ROYALBIT INFRASTRUCTURE (kveldulf)                       │
│                         Full Control - No 3rd Party Access                  │
│                                                                             │
│  ┌─────────────────────┐     ┌─────────────────────────────────────────┐   │
│  │  GITOLITE           │     │  ROYALBIT.CA                            │   │
│  │  royalbit/forge     │     │  Static Site (nginx:alpine-slim)        │   │
│  │                     │     │                                         │   │
│  │  • Full source code │     │  /forge/                                │   │
│  │  • Complete history │     │    forge-linux-x86_64                   │   │
│  │  • 330+ commits     │     │    forge-macos-arm64                    │   │
│  │  • ASIMOV velocity  │     │    forge-windows.exe                    │   │
│  │    proof            │     │    checksums.sha256                     │   │
│  │                     │     │                                         │   │
│  │  Protocol: SSH      │     │  /forge/docs/                           │   │
│  │  Access: @wheel     │     │    competitive-analysis.html            │   │
│  │                     │     │    screenshots/                         │   │
│  └─────────────────────┘     │                                         │   │
│                              │  Protocol: HTTPS (Traefik)              │   │
│                              │  Access: Public (downloads only)        │   │
│                              └─────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                    GITHUB (Marketing & Validation Only)                     │
│                         No Source Code - Public Visibility                  │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  royalbit/forge-demo                                                 │   │
│  │                                                                      │   │
│  │  Contents:                                                          │   │
│  │  • E2E test suite (Gnumeric/LibreOffice validation)                 │   │
│  │  • Example YAML financial models                                    │   │
│  │  • README with enterprise pitch                                     │   │
│  │  • Links to royalbit.ca/forge/ for binary downloads                 │   │
│  │  • GitHub Issues for enterprise inquiries                           │   │
│  │                                                                      │   │
│  │  NOT Included:                                                       │   │
│  │  • Source code                                                       │   │
│  │  • Build scripts                                                     │   │
│  │  • Internal documentation                                            │   │
│  │                                                                      │   │
│  │  License: Evaluation-Only (ultra-restrictive)                       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Infrastructure Components

### Gitolite (Source Code Repository)

| Property | Value |
|----------|-------|
| **Host** | kveldulf (Docker Swarm) |
| **Service** | `dev_gitolite` |
| **URL** | `git@crypto1.ca:royalbit/forge.git` |
| **Access** | SSH key authentication |
| **Permissions** | @wheel group (RW+) |

**Security:**
- No public access
- No web interface
- SSH-only protocol
- Key-based authentication
- Air-gapped from internet (except SSH)

### RoyalBit.ca (Binary Distribution)

| Property | Value |
|----------|-------|
| **Host** | kveldulf (Docker Swarm) |
| **Service** | `site_royalbit` |
| **URL** | `https://royalbit.ca/forge/` |
| **Image** | `nginx:alpine-slim` |
| **Proxy** | Traefik 3 (SSL termination) |

**Contents:**
```
/forge/
├── forge-linux-x86_64          # Linux binary
├── forge-macos-arm64           # macOS Apple Silicon
├── forge-macos-x86_64          # macOS Intel
├── forge-windows.exe           # Windows binary
├── checksums.sha256            # Verification
└── docs/
    ├── index.html              # Landing page
    ├── competitive-analysis/   # Market positioning
    └── screenshots/            # Product demos
```

### GitHub (Public Demo Repository)

| Property | Value |
|----------|-------|
| **Repository** | `royalbit/forge-demo` |
| **Visibility** | Public |
| **Purpose** | Marketing, validation, enterprise leads |

**Contents:**
- E2E test suite (proves formula accuracy)
- Example YAML models (shows capabilities)
- README (enterprise pitch)
- Issues (lead capture)

**NOT Included:**
- Source code
- Cargo.toml / build configuration
- Internal documentation
- Development tooling

## Data Flow

```
Development:
  Developer → SSH → Gitolite (royalbit/forge)
                         │
                         ▼
                    cargo build --release
                         │
                         ▼
                    Binary artifacts
                         │
            ┌────────────┴────────────┐
            ▼                         ▼
    royalbit.ca/forge/        GitHub forge-demo
    (binary downloads)         (E2E tests only)
```

## Security Model

### Source Code Protection

| Threat | Mitigation |
|--------|------------|
| GitHub AI training | Source not on GitHub |
| Microsoft access | Source not on GitHub |
| Public disclosure | Private gitolite repo |
| Unauthorized access | SSH key + @wheel group |

### Binary Distribution

| Threat | Mitigation |
|--------|------------|
| Tampered binaries | SHA256 checksums |
| Man-in-the-middle | HTTPS via Traefik |
| Unauthorized hosting | Ultra-restrictive license |

### Public Demo

| Threat | Mitigation |
|--------|------------|
| Code theft | No source code in repo |
| Reverse engineering | Binaries on separate domain |
| License violation | Evaluation-only terms |

## Why This Architecture?

### Problem: GitHub + Microsoft + Copilot

Research (December 2025) revealed:
- GitHub free tier has NO clear AI training opt-out
- Copilot lawsuit ongoing ($1B class action)
- ToS Section D grants broad rights to GitHub
- Private repos "not trained on" but murky for free tier

See: [ADR-011: Source Code Closure](architecture/ADR-011-SOURCE-CODE-CLOSURE.md)

### Solution: Self-Hosted + Hybrid

1. **Full Control**: Source lives on infrastructure we own
2. **Git History Preserved**: ASIMOV velocity proof intact
3. **Public Presence**: GitHub for discoverability
4. **Binary Distribution**: Our domain, our terms
5. **Zero Cost**: Using existing infrastructure

## Git Remote Configuration

```bash
# Forge repository remotes
$ git remote -v
origin    git@crypto1.ca:royalbit/forge.git (fetch)
origin    git@crypto1.ca:royalbit/forge.git (push)
github    git@github.com:royalbit/forge.git (fetch)
github    git@github.com:royalbit/forge.git (push)

# Primary development → origin (gitolite)
# Public demo sync → github (forge-demo, separate repo)
```

## Deployment Process

### Source Code Updates

```bash
# Normal development workflow
git add .
git commit -m "feat: new feature"
git push origin main  # → gitolite
```

### Binary Release

```bash
# Build release binaries
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-pc-windows-gnu

# Generate checksums
sha256sum forge-* > checksums.sha256

# Upload to royalbit.ca
scp forge-* checksums.sha256 kveldulf:/path/to/royalbit.ca/html/forge/

# Update site
ssh kveldulf "cd /path/to/sites/royalbit.ca && ./build-push-image && docker service update site_royalbit"
```

### Demo Repository Update

```bash
# Extract E2E tests and examples (no source)
# Push to GitHub forge-demo repository
# This is a SEPARATE repo, not the source repo
```

## References

- [ADR-011: Source Code Closure](architecture/ADR-011-SOURCE-CODE-CLOSURE.md)
- [Competitive Analysis](COMPETITIVE_ANALYSIS.md)
- [GitHub ToS Research](architecture/ADR-011-SOURCE-CODE-CLOSURE.md#github-tos-analysis)
