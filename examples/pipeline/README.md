# A repository of models, checked on every pull request

[`forge.yml`](forge.yml) is a GitHub Actions workflow. Copy it to `.github/workflows/forge.yml` in a repository whose models are at its top. On every pull request it fails when a stored result is out of date or a formula has no stored result (`forge validate`, from 10.0.0-rc.35), and it shows each model's results on the run's page.

[The guide's pipeline page](../../docs/guide/pipeline.md) explains each step, and links the runs where it passed and failed.
