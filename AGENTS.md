# lampy

<!-- TODO: what it is — one paragraph (what/why/boundary in
brief). Naming note if the repo is named for something. -->

## Layout

<!-- TODO: crate/module table or map — must match source
exactly; generate where a generator exists. -->

## Commands

<!-- TODO: build/test/dev commands (cargo xtask …, bun …). -->

## Boundary

<!-- TODO: what this repo owns vs its neighbors — what does
NOT belong here. -->

## Rules

<!-- TODO: lint/test/convention rules agents must follow. -->

## Docs

`docs/` worktree — VitePress + Diátaxis (getting-started,
how-to, reference, explanation)
<!-- TODO: public repos deploy to GitHub Pages at
https://9aia.com/lampy/; private repos keep the site
source, no deploy. `docs/package.json` carries the
`docs:check` drift gate — generated reference must not
drift from source. -->

<!-- myco:agent-awareness -->

## Agent awareness

- Session start / any work discussion: run `myco status --prompt`
  before proposing next steps — know the board first.
- User describes new work → offer to capture it (`myco todo add`,
  `issue new`, `proposal new`). Don't wait to be asked.
- Picking up a task: `myco resume <ID>` is the briefing.
- Wrapping up: `myco handoff <ID>` anything resumable; keep statuses
  honest (claimed → in-progress → blocked/done).

<!-- /myco:agent-awareness -->

<!-- sturn:agent-awareness -->

## Sturn awareness

- **No PRs.** The pushed branch/commit is the deliverable;
review streams continuously (sturn WAL), merge is a
human/sturn call.
- sturn-wired repo (`sturn.toml` present): claim before edits —
`sturn claim "<glob>" --ws <ws>` surfaces conflicts in ms.
- Ops via `sturn write|rename|delete`; `sturn snap` folds the ws
into a real commit; `sturn push` publishes heads +
refs/sturn/*.
- `sturn leave` on exit — drops the actor's claims.

<!-- /sturn:agent-awareness -->

<!-- kuda:agent-awareness -->

## Kuda awareness

- `.kuda/` cards are committed artifacts — read them before
re-exploring (`kuda map`, `kuda card <mod>`); `kuda status`
reports per-card drift vs HEAD.
- Code changes that invalidate a card → regenerate it
(`kuda sync`) in the same change; cards ride the diff they
describe.
- `.kuda/index.sqlite` is gitignored + rebuildable; the `*.yaml`
cards are the checked-in source of truth.

<!-- /kuda:agent-awareness -->
