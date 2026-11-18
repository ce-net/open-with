# Proof: slice 1 of CE-REF-OPENERS-AND-HOME (2026-11-18, foreman-ops-1)

Built and tested on foreman-ops-1 (cargo test: open-with 15, home-shell 4, ce-net-net 13, apps-shell 29), installed
there with `ce cell start/restart`, behind the public gate route `/home/` (repo:ce-fleet 4bfabd85, snapshot
`4bfabd85e82a-16c2dd95730f` on debian, foreman-ops-1, leif-desk). Test persons only, in apps.shell's people file with
an end 5 h out, and test sessions (`via=test`) for the browser:

- `t-home-all`: Leif's apps.shell abilities plus `home.shell:open`, `home.shell:links`, `open.with:use` ("tester");
- `t-home-promises`: `home.shell:open`, `open.with:use`, `crm.promises:read` ("tester-2");
- `t-home-stranger`: `search.hybrid:find` only.

Browser: headless Chromium (page-record) on `https://apps.ce-net.com/home/`, 1440x900 and 390x844. Shots and
videos stay on foreman-ops-1 in `~/.cache/page-record/p1-tester-*`, `p1-phone-*`, `p2-tester2-*`, `p3-stranger-*`,
`p6-stopped-*` (they show CRM records, so they are not in this public repo).

| # | Check | Measured |
|---|---|---|
| 1 | tester: Home slots | Today: 4 slots (promises 53, approve 30, cells, links 11); calls is absent because crm.calls has 0 calls today, the emergent rule. With tomorrow's window (`/api/layout?from=`, 6 calls) 5 slots: promises, calls, approve, cells, links. Layout 34-51 ms warm, 3.0 s on the first, cold call |
| 1 | one click per type opens the named view in the pane | assembly:huebot -> apps.shell::Port (huebot 3D, step 0 of 12, framed same origin); map:twinton -> Port (maps.ce-net.com framed); person:daniel-arm -> ref.view::Record (crm.people); promise:... -> Record (crm.promises); repo:microduck-ce/GET-ALL-PARTS-2026-11-07.md (doc/markdown) and repo:open-with/README.md -> ref.view::Text, pinned to the commit with sha256; cell:apps.shell -> site.cellmap::Cell; call:c-1 -> Record (crm.calls answers NotFound); web:https://example.com/x -> apps.shell::Outside ("outside ce-net", hand-off button); repo:open-with/tests/fixtures/sample.bin -> ref.view::Raw (41 B, sha256 2348eb1a...) |
| 1 | bar and back | the bar shows the short ref each time (`assembly:huebot`, `map:twinton`, `repo:open-with/README.md@595aebd...`); the address is `/home/o/<ref>`; history.back() returned to the README pane with its ref in the bar |
| 2 | tester-2 | tab bar: Home only; Home: 1 slot (Promises due); Open of `call:c-1`: `{"Refused":{"need":"crm.calls:read","ask":"view:approve.inbox@1::Request?need=crm.calls:read"}}`, no opener name; the pane says "it needs crm.calls:read" with an ask chip |
| 3 | stranger | `/api/open` 403, `/api/ref` 403, `/api/layout` 403, `/api/links` 403, `/v/home.js` 404; unsigned 401; the page shows no slot and "Opening refs is not open to you here" |
| 4 | unknown | `repo:nope/x.rs` -> Unknown "nope has no checkout on foreman-ops-1 (where ref.pin reads)"; `repo:apps-shell/no-such-file.txt` -> Unknown "node:foreman-ops-1 has no apps-shell/no-such-file.txt at a7290bd..." (ref.resolve's words); `bogus:x` -> Unknown "no kind \"bogus\" in the kind table"; the `.bin` opens in Raw |
| 5 | lint | apps-shell `tests/views.rs` passes over 17 view modules; in the browser on every step: `a[target=_blank]` 0, `a[href^=http]` 0, 87 to 89 chips |
| 6 | open.with stopped | pane: "open.with is not answering (open.with@1 not reachable ...). Trying again every 3 s." with "Show raw from cache"; started 30 s later: the pane showed the Port view of assembly:huebot 17 s on, same page load (navigation type `navigate`, no reload). API: 503 stopped, 200 on the second try after start |
| 7 | schema hashes | unchanged: apps.shell 0eff16cb36c9fc92, crm.calls 75a354c0af4a012f, crm.people 448ba80c84fc9d29, crm.promises b536f482fae0a492, ref.pin 966357c40f1b9fff, ref.resolve 0aa2e2474931699b, ref.graph 6a80238adaff0379, site.cellmap 7af95a8f859a6425 |

Found while proving: net.find files a replica under its door registry version, which is 1 for crm.calls although its
api is major 2, so `Find(crm.calls, 2)` answered NoStartSpec and the call opener was taken for down; `live()` now also
accepts a replica filed under 1 when the line's major finds none.
