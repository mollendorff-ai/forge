### Changed

- **The repository checks (`validation/repo`) are covered 100%, and CI holds that floor.** What was untested was the failing half of each check, the code that only runs when the repository breaks a rule: its message, and what it counts. Each check now takes its input as an argument and is tested on the repository, which passes, and on a made-up input that breaks the rule, which fails with the message. The three floors of the Repository checks job are now 100% lines, functions and regions. No behaviour of Forge changes.
