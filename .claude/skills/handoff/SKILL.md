---
name: handoff
description: Decide whether to continue in this session or start a fresh one (/clear), and when a fresh one is better, write the handoff prompt for it. Use when the user asks "new session or continue?", "what's best, /clear or keep going?", or asks for a handoff prompt.
---

# Handoff: continue here, or start fresh?

## 1. Decide, and say which in the first line

Recommend a **fresh session** when any of these holds:

- The next step is a different kind of work from what this session did (another
  crate or language, a new CI suite, a different roadmap item), so almost none
  of the conversation's code is needed.
- The conversation is long: several features, merges, CI rounds or compactions.
  A short brief reasons better than a long history, and each turn costs less.
- The next step wants a different model or effort than this session runs on
  (CLAUDE.md: give each step its model and effort, and say why).

Recommend **continuing here** when the next step edits the files just written,
needs details that would be lost in a summary (an open debugging thread, a
half-done refactor), or is a small follow-up (a CI fix, a review comment).

Give the reason in one or two sentences. Do not hedge between the two.

## 2. If fresh: write the handoff prompt

Facts only, checked against the repository now (run `git status`, `git log`,
the CI check for the head commit), never from memory. A new session knows
nothing, so the prompt must stand alone. Put it in one code block the user can
paste after `/clear`, with these parts in this order:

1. **Where:** repository, branch (pushed? fetch it), the open PR and whether
   it is clean, and the first thing to do (read CLAUDE.md and the named docs
   and roadmap items).
2. **State:** what is built, by file; what CI said on the head commit,
   including coverage against the floor; what is deliberately left.
3. **Next, in order:** each step with its acceptance, taken from the roadmap
   item, and the model and effort to run it with and why.
4. **Lessons:** every trap this session hit, each with its fix (coverage test
   helper pitfalls, how to measure coverage, clippy on the oldest Rust, the
   README test-count rule, large-file limits, commit hook time, how to read
   CI). Leave out anything that was only true of one dead end.
5. **Numbers and tools:** reference values already checked and by what tool and
   version, and where the tool is installed or how to install it.
6. **Rules:** restate the non-negotiables from CLAUDE.md in one line, and who
   may push where.

Keep it under about 80 lines. Link the files that hold the detail instead of
copying them.

## 3. Optionally write it to a file

If the user wants the prompt kept, write it to the scratchpad directory and
give the full path. Do not commit it unless asked.
