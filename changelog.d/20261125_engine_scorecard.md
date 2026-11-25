### Added

- **An OpenSSF Scorecard of the repository, and its badge** (roadmap A17-SCORECARD). `.github/workflows/scorecard.yml` rates how the repository is run (pinned actions, a security policy, signed releases and the rest) weekly and on every push to `main`, publishes the score, and uploads the findings to the Security tab; the README shows the badge. Both actions it uses are pinned to a commit, and a test in `validation/repo` now fails when any action in any workflow is not.
