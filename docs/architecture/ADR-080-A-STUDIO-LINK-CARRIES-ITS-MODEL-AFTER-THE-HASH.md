# ADR-080: A Studio Link Carries Its Model After the #, and Nowhere Else

**Status:** Accepted by the maintainer (2026-11-24); built
**Date:** 2026-11-24
**Decided by:** the maintainer, who asked for the roadmap item with an ADR first (A7-STUDIO-LINK) and accepts this by merging it; the analysis is Claude's, in a session configured for Opus 5.5
**Closes:** roadmap A7-STUDIO-LINK

---

## Context

A link to Forge Studio can open only a bundled example: `?example=bagel`, and `&model=price_war.yaml` for one model of a set (ADR-077). A lesson in `docs/guide/` or an AI's answer in a chat cannot hand over its own model. The reader has to copy the YAML, save it as a file and open it from Files, which a phone makes hard.

Studio promises that nothing you type leaves the tab (README, `studio/README.md`, the welcome card, ADR-056 decision 5). A link that carries a model must keep that promise.

A URL has two places a model could go:

- **The query (`?…`)** goes to the server with the request. GitHub Pages, any proxy on the way, and their logs would see the model.
- **The fragment (`#…`)** never leaves the browser. A request never carries it, not to the page's own server and not in the `Referer` header. Studio's service worker (`sw.js`) may see it in a navigation's URL, but only matches that against the files it keeps, and asks the server for a file it does not have with a request that, like every request, has no fragment.

A link also changes who puts text into Studio. Until now, only the person at the device did, by typing or by opening a file. With a link, anyone who sends one does. Studio draws a model's names, text values and messages into the page. If one of them got through as HTML, and a script ran, it could read the browser's storage and send the person's other files elsewhere. Studio escapes what it draws (`esc` in `text.js`), but nothing yet stops a script that gets past a missed escape.

The models are small. Once compressed, the bagel shop's main file is a 1,832-character link and the starter model 1,088. Kittiwake's deal, at 10,905 bytes, comes to 4,566 characters compressed and 16,999 plain.

## Options

1. **The model in the query**, as `?example=` is. The model leaves the device with every request, which breaks the promise. Rejected.
2. **The model in the fragment, as plain text** (percent-encoded YAML). Anyone can write the link, an AI included, with no tool and no arithmetic it could get wrong. But it is long: about 1.55 times the file.
3. **The model in the fragment, compressed**: gzip, then base64url. About a third of the plain link, made with what every browser has (`CompressionStream`) and every system has (`gzip`, `base64`). A person or an AI cannot write it without running something.
4. **A short link to a stored model** (a paste service, a gist). Short, but the model leaves the device and is kept by someone else. Rejected for the same reason as option 1.

## Decision

Options 2 and 3, both read, with option 3 written by Studio:

1. **A model travels only after the `#`.** Studio reads a model from the fragment and nowhere else. The query keeps only `?example=` and `&model=`, which name a bundled example and carry no text of the person's.

2. **The fragment is a list of `key=value` parts joined by `&`:**
   - `name=` the file's name (percent-encoded) for the next file in the list. Without one, a file is `model.yaml`.
   - `yaml=` a file's text, percent-encoded (`encodeURIComponent`). This is the form a person or an AI writes by hand.
   - `gzip=` a file's text as UTF-8, gzipped, in base64url without padding. This is the form Studio writes. From a terminal: `gzip -9c model.yaml | base64 | tr '+/' '-_' | tr -d '=\n'`.

   The first file is the main model; the files after it are what it includes, so `name=bagel.yaml&gzip=…&name=costs.yaml&gzip=…` opens the bagel shop with its costs. A file's name is a plain file name ending in `.yaml` or `.yml`, with no folder.

3. **A fragment with none of these keys is not a model**: `#an-sim`, which Studio uses inside the page, is left alone. A fragment that has one of them is read whole or not at all. An unknown key, two files of the same name, a name that is not a plain YAML file name, text that does not decode, or more than 2,000,000 bytes of text once decoded: Studio says what is wrong and keeps what was open.

4. **Opening a link does not lose work.** When the open files are the person's own (opened from the device, imported or from an earlier link), or an example edited, Studio asks first and names the files the link would replace. An example as it ships is replaced without asking, since Files can always open it again. The opened model is kept in the browser's storage, as a file opened from the device is.

5. **The link is removed from the address bar once read** (`history.replaceState`), the query with it, so a reload shows the model as last edited, not as the link had it. A link pasted into the address bar of a Studio tab that is already open (`hashchange`) opens the same way.

6. **Files makes a link to the open model**: the main file and the files it includes, in the `gzip=` form. It copies the link to the clipboard where the browser allows, and shows it in a field to copy by hand where it does not. Files says what a link is: anyone who has it has the model, and it stays in the browser's history like any address. The link leaves the tab only when the person pastes it somewhere.

7. **The page has a Content Security Policy** (a `<meta>` in `index.html`, since GitHub Pages sends no headers). Scripts, workers and every request are limited to the page's own address: `default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'`. A script injected through a missed escape does not run, since inline scripts are not allowed. Even if one did, it could reach no other server. The promise then rests on the browser, not only on every escape being right. `'unsafe-inline'` for styles stays, because the cards set `style=` attributes. A style cannot run code, and with `img-src` and `connect-src` limited to the page, it cannot send anything either.

## Consequences

- A lesson or an AI can hand over a model with a link. An AI with no tools writes the `yaml=` form. Studio, a script or `gzip` writes the shorter one. `docs/guide/ai.md` says how.
- "Nothing you type leaves the tab" stays true and gets more exact: the README and `studio/README.md` say that a link's model is after the `#`, which no server receives, and that a link you make holds your model wherever you paste it.
- The privacy promise is tested, not only written. `studio/tests/link.mjs` records every request the page makes to its server and fails if one carries any part of the model. It opens the mortgage, the bagel shop with its costs and the starter model from links, and finds every result equal to `forge calculate`'s (`forge-wasm/expected.json`). It also checks that text a link carries is drawn as text, that the policy stops an inline handler and a request to another server, and that it blocks nothing Studio itself uses (every Studio test runs under it).
- A link is as private as the place it is pasted. A model sent in a chat is in that chat, and a link opened is in the browser's history. Files says so next to the link.
- A link is limited by what carries it, not by Studio. Most browsers accept a link of several megabytes. Some chat apps and mail programs cut a link at a few thousand characters, which is why Studio writes the compressed form.
- `forge` could print a link for a model (`forge studio model.yaml`) and the MCP server could answer one. That needs gzip and base64 on the Rust side and is left for later; the format above is what it would write.
- The policy is new for every page of Studio. A future feature that needs another origin (a font, an image from elsewhere) must change it here, in an ADR, and say how the promise still holds.
