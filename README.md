# open-with

`open.with@1`: which cell's view opens a ce-ref, for this viewer (repo:ce/docs/CE-REF-OPENERS-AND-HOME-2026-11-18.md
section 3.2, slice 1 in section 8 A). One repo, one cell, one function.

- `crates/ce-ref-text`: the two written forms of one ce-ref. Short `kind:target[@pin][#frag]` (prose, the address bar)
  and full `ce-ref://kind/target[@pin][#frag]` (clipboard, deep link, mail) parse to one `CeRef`; the portable
  `ce-ref://repo/<sha256-origin>/<path>` round-trips byte for byte. `type_of` gives the type string openers match: the
  kind for record and structural kinds, the media from the extension for `repo` (`code/rust`, `code/js`, `text/plain`,
  `doc/markdown`, `image/png`, else `*`). `find` returns the refs written in a text. Moves into `ce-ref-id` in step 8.
- `crates/open-with-api`: ops `Open{r, mode, device} -> Target{cell, major, view, props, r, ty, others} |
  Refused{need, ask} | NoOpener{ty} | Unknown{reason}`, `Handlers{ty, mode}`, `HealthOp`; `Peek`, `Prefer`, `Changes`
  answer `NotYet`.
- `crates/open-with`: the opener table is repo:ce-net-net `opens()` (cell-net takes no `opens` lines yet). Filter: the
  viewer's chain holds the line's `needs`; the line's cell answers on a replica (net.find, cached 5 s). Rank: most
  specific pattern, then rank (ties fail ce-net-net's `cargo test`). A refused viewer gets the right it lacks and an ask
  ref, never an opener's name. A `repo:` ref is resolved first by ref.resolve (no commit: this node's checkout HEAD);
  an unknown one answers `Unknown` with the resolver's words.

Callers: apps.shell `GET /api/open?r=` (the viewer's grant, minted by wire.ws-door for exactly that viewer).
Keys: `<ce data dir>/cell-keys/open.with/` (`ref.resolve:read`, `net.find:find`, `delegate:1`).
`cargo test`: 15 (9 grammar, 6 choice). `tests/fixtures/sample.bin` is the `.bin` the proof opens in Raw.
Proof: [PROOF.md](PROOF.md). Apache-2.0.
