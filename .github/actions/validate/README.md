# Forge Validate Action

Validate Forge YAML formulas in your CI/CD pipeline. Catch formula errors before they reach production.

## Usage

### Basic Usage

```yaml
- uses: royalbit/forge/.github/actions/validate@main
  with:
    files: '**/*.forge.yaml'
```

### Full Example

```yaml
name: Validate Financial Models

on:
  push:
    paths:
      - '**.yaml'
      - '**.yml'
  pull_request:
    paths:
      - '**.yaml'
      - '**.yml'

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Validate Forge Models
        uses: royalbit/forge/.github/actions/validate@main
        with:
          files: 'models/**/*.yaml'
          fail-on-error: true
```

## Inputs

| Input | Description | Required | Default |
|-------|-------------|----------|---------|
| `files` | Glob pattern for YAML files to validate | No | `**/*.forge.yaml` |
| `fail-on-error` | Fail the action if validation errors are found | No | `true` |
| `working-directory` | Working directory for validation | No | `.` |
| `rust-version` | Rust toolchain version to use | No | `stable` |

## Outputs

| Output | Description |
|--------|-------------|
| `validated` | Number of files validated |
| `errors` | Number of validation errors found |
| `status` | Validation status (`success` or `failure`) |

## Examples

### Validate specific directory

```yaml
- uses: royalbit/forge/.github/actions/validate@main
  with:
    files: 'financial-models/*.yaml'
```

### Continue on error (for reporting)

```yaml
- uses: royalbit/forge/.github/actions/validate@main
  id: validate
  with:
    files: '**/*.yaml'
    fail-on-error: false

- name: Report validation results
  run: |
    echo "Validated: ${{ steps.validate.outputs.validated }} files"
    echo "Errors: ${{ steps.validate.outputs.errors }}"
    echo "Status: ${{ steps.validate.outputs.status }}"
```

### Use in PR checks

```yaml
name: PR Validation

on:
  pull_request:
    branches: [main]

jobs:
  validate-models:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Validate Financial Models
        uses: royalbit/forge/.github/actions/validate@main
        with:
          files: 'models/**/*.yaml'
          fail-on-error: true

      - name: Comment on PR
        if: failure()
        uses: actions/github-script@v7
        with:
          script: |
            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: '## Formula Validation Failed\n\nPlease run `forge validate` locally and fix any errors before merging.'
            })
```

## What it validates

- Formula syntax correctness
- Circular dependency detection
- Value/formula consistency (calculated values match stored values)
- Cross-file reference resolution
- Type safety for arrays

## Performance

The action caches the Cargo registry and Forge binary for fast subsequent runs.
Typical validation time: <30 seconds for first run, <5 seconds for cached runs.

## License

MIT - RoyalBit Inc.
