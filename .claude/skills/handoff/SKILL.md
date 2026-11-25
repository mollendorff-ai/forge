---
name: handoff
description: Decide whether to continue in this session or start a fresh one (/clear), and when a fresh one is better, write the handoff prompt for it, for the same lane when the project works in lanes. Use when the user asks "new session or continue?", "what's best, /clear or keep going?", or asks for a handoff prompt.
---

# Handoff: continue here, or start fresh?

Some projects are worked on by several sessions at once, each in its own
lane (`lanes:` in `.asimov/project.yaml`; the project's `AGENTS.md` says what
each owns). Then a handoff passes one lane's work to that lane's next session
and never carries another lane's work: what is next is the lane's roadmap
(`.asimov/roadmap/<lane>.yaml`), and how to work in the project is its rules
file and sessions page, which the next session reads anyway. A project with no
lanes has one roadmap, `.asimov/roadmap.yaml`, and one list of lessons: read
"the lane" below as "the project".

## 1. Decide, and say which in the first line

Recommend a **fresh session** when any of these holds:

- The next step is a different kind of work from what this session did (another
  crate or language, a new CI suite, a different roadmap item), so almost none
  of the conversation's code is needed.
- The conversation is long: several features, merges, CI rounds or compactions.
  A short brief reasons better than a long history, and each turn costs less.
- The next step wants a different model or effort than this session runs on
  (give each step its model and effort, and say why).

Recommend **continuing here** when the next step edits the files just written,
needs details that would be lost in a summary (an open debugging thread, a
half-done refactor), or is a small follow-up (a CI fix, a review comment).

Give the reason in one or two sentences. Do not hedge between the two.

## 2. Before writing it: put what lasts where it lasts

A handoff is read once; the repository is read by every session. Before
writing the prompt, on this session's branch:

- **Findings for this lane** go in its roadmap as backlog items with an
  acceptance, or on the item they belong to.
- **Findings for another lane** go at the end of that lane's backlog. Never
  only in the handoff: the other lane's session will not read it.
- **A finished item** moves to the lane's `done:` list with a dated `built:`.
- **A lesson that holds for every session** (a command, a trap and its fix)
  goes in the project's sessions page (`docs/SESSIONS.md` in Forge) or its
  rules file, whichever the rules file names.

## 3. If fresh: write the handoff prompt

Facts only, checked against the repository now (run `git status`, `git log`,
the CI check for the head commit), never from memory. Put it in one code block
the user can paste after `/clear`, with these parts in this order:

1. **Lane:** the first line, `Lane: <id>`, then: read the rules file
   (`AGENTS.md`), then the lane's roadmap on `main`. Without lanes: read the
   rules file and the roadmap.
2. **Where:** the branch (pushed? merged?), the open PR, whether it is clean
   and what CI said on its head commit, and the first thing to do.
3. **In flight:** only what is not on `main` yet: work half done, by file; a
   decision the maintainer has not made yet; a check still running.
4. **Next:** the roadmap item ids to take, in order, each with the model and
   effort to run it with and why. The acceptance is in the roadmap: do not
   copy it.
5. **New lessons:** only those not yet in the sessions page and not for it
   (true of this work alone). Usually none.

Keep it under about 30 lines. A longer one is carrying what belongs in the
roadmap or the sessions page: move it there first.

## 4. Optionally write it to a file

If the user wants the prompt kept, write it to the scratchpad directory and
give the full path. Do not commit it unless asked.
