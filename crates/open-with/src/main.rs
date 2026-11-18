//! `open-with serve`: the open.with@1 cell behind this machine's door.
//!
//! `Open` for one viewer (the caller's chain is the viewer grant apps.shell had wire.ws-door mint for exactly that
//! viewer): the kind must be in the kind table; a `repo:` ref is first resolved by ref.resolve (no commit: the HEAD of
//! this node's checkout, where ref.pin reads) and an unknown one answers `Unknown` with the resolver's own words; then
//! the opener is chosen by lib.rs `choose` over repo:ce-net-net `opens()`, filtered by the viewer's chain and by
//! whether each opener's cell answers on a replica (net.find, cached 5 s). The name of an opener the viewer may not use
//! never leaves this cell.
//!
//! This cell's own key (`<ce data dir>/cell-keys/open.with/`) holds `ref.resolve:read` (resolving existence, place and
//! hash, never the bytes: the view fetches those as the viewer), `net.find:find` and `delegate:1`.
use ce_wire::Server;
use open_with::*;
use open_with_api as api;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const CELL: &str = "open.with";
/// How long one liveness answer is trusted.
const LIVE_TTL: Duration = Duration::from_secs(5);
/// A replica whose health op answered within this long is live.
const HEALTHY_MS: u32 = 30_000;

/// ref.resolve's contract (repo:live-refs contracts/ref_resolve.rs, major 1), copied verbatim as in repo:apps-shell.
#[allow(dead_code)]
mod ref_resolve_api {
    ce_wire::api! {
    api RefResolve = "ref.resolve", version 1;
    types {
        /// Health contract: these three fields, in this order, in every cell.
        pub struct Health { pub ok: bool, pub version: String, pub uptime_s: u64 }
        pub struct ResolveReq { pub reference: String }
        /// One place the thing is now: the node, its key, and what it holds there (a content hash for bytes).
        pub struct Holder { pub node: String, pub node_key: String, pub sha256: Option<String>, pub size: u64, pub since_ms: u64, pub live: bool }
        /// `source` is "mesh" when a holder answered now, "local index" when none could be reached; `as_of_ms` is
        /// then the time of the newest announcement the local index holds.
        pub struct Located { pub id: String, pub kind: String, pub holders: Vec<Holder>, pub pin: Option<String>, pub source: String, pub as_of_ms: u64, pub detail: String }
        pub enum Resolution { Located(Located), Ambiguous { written: String, reason: String, candidates: Vec<String> }, Unknown { written: String, reason: String } }
        pub struct Announce { pub id: String, pub kind: String, pub node: String, pub sha256: Option<String>, pub size: u64, pub at_ms: u64, pub gone: bool, pub detail: String }
        pub enum RefError { Invalid(String), Unavailable(String) }
    }
    ops {
        1 => HealthOp(()) -> Health, needs "ref.resolve:health";
        2 => Resolve(ResolveReq) -> Result<Resolution, RefError>, needs "ref.resolve:read";
        3 => Publish(Announce) -> Result<u64, RefError>, needs "ref.resolve:announce";
    }
    }
}

type LiveCache = Arc<Mutex<BTreeMap<(String, u16), (bool, Instant)>>>;

fn mesh_token() -> Vec<u8> {
    ui_chain::source_token::<ce_wire::find::NetFind>(&["net.find:find"], 60, None).unwrap_or_default()
}

/// Does `cell@major` answer on a replica? net.find's healthy replicas of that major, cached [`LIVE_TTL`].
async fn live(cache: &LiveCache, cell: &str, major: u16) -> bool {
    let key = (cell.to_string(), major);
    if let Some((ok, at)) = cache.lock().unwrap().get(&key) {
        if at.elapsed() < LIVE_TTL {
            return *ok;
        }
    }
    let ok = match tokio::time::timeout(Duration::from_millis(2500), ce_wire::find::find_async(cell, major)).await {
        Ok(Some(Ok(r))) => r.replicas.iter().any(|x| x.healthy_ms <= HEALTHY_MS),
        _ => false,
    };
    cache.lock().unwrap().insert(key, (ok, Instant::now()));
    ok
}

/// Liveness of every cell the candidates for `ty` name, asked at once.
async fn live_map(cache: &LiveCache, table: &[Opens], ty: &str, mode: &api::Mode) -> BTreeMap<(String, u16), bool> {
    let mut want: Vec<(String, u16)> = candidates(table, ty, mode).iter().map(|o| (o.cell.to_string(), o.major)).collect();
    want.dedup();
    let hs: Vec<_> = want
        .into_iter()
        .map(|(c, m)| {
            let cache = cache.clone();
            tokio::spawn(async move {
                let ok = live(&cache, &c, m).await;
                ((c, m), ok)
            })
        })
        .collect();
    let mut out = BTreeMap::new();
    for h in hs {
        if let Ok((k, ok)) = h.await {
            out.insert(k, ok);
        }
    }
    out
}

fn home() -> String {
    std::env::var("HOME").unwrap_or_default()
}

/// The `repo:` ref with the commit this node's checkout is at, when it has none. Err: the resolver's words.
async fn with_commit(r: &CeRef) -> Result<CeRef, String> {
    if r.pin.is_some() {
        return Ok(r.clone());
    }
    let repo = r.target.split('/').next().unwrap_or("");
    if repo.is_empty() || repo.starts_with('.') || !repo.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b)) {
        return Err(format!("{r}: not repo:<repo>/<path>"));
    }
    let o = tokio::process::Command::new("git").arg("-C").arg(format!("{}/dev/{repo}", home())).args(["rev-parse", "HEAD"]).output().await;
    match o {
        Ok(o) if o.status.success() => {
            let mut p = r.clone();
            p.pin = Some(ce_ref_text::Pin::Commit(String::from_utf8_lossy(&o.stdout).trim().to_string()));
            Ok(p)
        }
        _ => Err(format!("{repo} has no checkout on {} (where ref.pin reads)", ui_chain::hostname())),
    }
}

enum Resolved {
    Ok { r: CeRef, sha256: Option<String> },
    Unknown(String),
}

async fn resolve_repo(r: &CeRef) -> Resolved {
    use ref_resolve_api::*;
    let r = match with_commit(r).await {
        Ok(r) => r,
        Err(e) => return Resolved::Unknown(e),
    };
    let written = CeRef { frag: None, ..r.clone() }.to_string();
    let tok = match ui_chain::source_token::<RefResolve>(&["ref.resolve:read"], 60, None) {
        Ok(t) => t,
        Err(e) => return Resolved::Unknown(format!("open.with cannot ask ref.resolve: {e}")),
    };
    let c = match tokio::time::timeout(Duration::from_secs(4), ce_wire::Client::<RefResolve>::by_name(tok)).await {
        Ok(Ok(c)) => c,
        _ => return Resolved::Unknown("ref.resolve did not answer in 4 s".into()),
    };
    match c.call::<Resolve>(&ResolveReq { reference: written.clone() }).await {
        Ok(Ok(Resolution::Located(l))) => Resolved::Ok { sha256: l.pin.map(|p| p.trim_start_matches("sha256:").to_string()), r },
        Ok(Ok(Resolution::Unknown { reason, .. })) => Resolved::Unknown(reason),
        Ok(Ok(Resolution::Ambiguous { reason, .. })) => Resolved::Unknown(reason),
        Ok(Err(e)) => Resolved::Unknown(format!("ref.resolve: {e:?}")),
        Err(e) => Resolved::Unknown(format!("ref.resolve: {e}")),
    }
}

async fn open(cache: &LiveCache, req: api::OpenReq, who: &ce_wire::Caller) -> api::Opened {
    let r: CeRef = req.r.clone().into();
    if !KINDS.contains(&r.kind.as_str()) {
        return api::Opened::Unknown { reason: format!("no kind {:?} in the kind table", r.kind) };
    }
    let holds = |a: &str| ui_chain::holds(who, a);
    let (r, sha) = if ce_ref_text::BYTE_KINDS.contains(&r.kind.as_str()) && r.kind == "repo" {
        if !holds("ref.resolve:read") {
            return api::Opened::Refused { need: "ref.resolve:read".into(), ask: ask_ref("ref.resolve:read").into() };
        }
        match resolve_repo(&r).await {
            Resolved::Ok { r: resolved, sha256 } => (CeRef { frag: r.frag.clone(), ..resolved }, sha256),
            Resolved::Unknown(reason) => return api::Opened::Unknown { reason },
        }
    } else {
        (r, None)
    };
    let ty = type_of(&r);
    let table = ce_net_net::opens();
    let lm = live_map(cache, &table, &ty, &req.mode).await;
    let is_live = |c: &str, m: u16| lm.get(&(c.to_string(), m)).copied().unwrap_or(false);
    match choose(&table, &ty, &req.mode, &holds, &is_live) {
        Choice::Target { opener, others } => api::Opened::Target {
            cell: opener.cell.into(),
            major: opener.major,
            view: opener.view.into(),
            props: props(&r, &opener, sha.as_deref()),
            r: r.into(),
            ty,
            others,
        },
        Choice::Refused { need } => api::Opened::Refused { ask: ask_ref(&need).into(), need },
        Choice::NoOpener { ty } => api::Opened::NoOpener { ty },
        Choice::Down { ty } => api::Opened::Unknown { reason: format!("no opener of {ty} answers now; try again in a moment") },
    }
}

async fn serve() -> Result<(), String> {
    let me = ui_chain::init(CELL)?.root();
    ce_wire::find::set_mesh_token(mesh_token);
    let started = Instant::now();
    let cache: LiveCache = Arc::default();
    let (c1, c2) = (cache.clone(), cache.clone());
    let not_yet = |what: &str| api::OpenError::NotYet(format!("{what} comes after slice 1 (CE-REF-OPENERS-AND-HOME section 7)"));
    let gate = ce_gate::Gate::<api::OpenWith>::new(me, ce_gate::mesh_roots()).into_fn();
    Server::<api::OpenWith>::new(gate)
        .on::<api::HealthOp, _, _>(move |_: (), _| async move { Ok(api::Health { ok: true, version: env!("CARGO_PKG_VERSION").into(), uptime_s: started.elapsed().as_secs() }) })
        .on::<api::Open, _, _>(move |req: api::OpenReq, who| {
            let cache = c1.clone();
            async move {
                let t0 = Instant::now();
                let r = req.r.short();
                let out = open(&cache, req, &who).await;
                let what = match &out {
                    api::Opened::Target { view, .. } => format!("target {view}"),
                    api::Opened::Refused { need, .. } => format!("refused, needs {need}"),
                    api::Opened::NoOpener { ty } => format!("no opener of {ty}"),
                    api::Opened::Unknown { reason } => format!("unknown: {}", reason.chars().take(120).collect::<String>()),
                };
                eprintln!("open.with: {} {r}: {what} in {} ms", who.name, t0.elapsed().as_millis());
                Ok(out)
            }
        })
        .on::<api::Handlers, _, _>(move |req: api::HandlersReq, who| {
            let cache = c2.clone();
            async move {
                let table = ce_net_net::opens();
                let lm = live_map(&cache, &table, &req.ty, &req.mode).await;
                let is_live = |c: &str, m: u16| lm.get(&(c.to_string(), m)).copied().unwrap_or(false);
                let holds = |a: &str| ui_chain::holds(&who, a);
                Ok(usable(&table, &req.ty, &req.mode, &holds, &is_live)
                    .into_iter()
                    .map(|(o, up)| api::Handler { cell: o.cell.into(), major: o.major, view: o.view.into(), pattern: o.pattern.into(), rank: o.rank, live: up })
                    .collect())
            }
        })
        .on::<api::Peek, _, _>(move |_: api::PeekReq, _| async move { Ok(Err(not_yet("Peek"))) })
        .on::<api::Prefer, _, _>(move |_: api::PreferReq, _| async move { Ok(Err(not_yet("Prefer"))) })
        .on::<api::Changes, _, _>(move |_: api::ChangesReq, _| async move { Ok(Err(not_yet("Changes"))) })
        .audit(ce_gate::audit::file())
        .serve_door()
        .await
        .map_err(|e| e.to_string())
}

fn main() {
    let r = match std::env::args().nth(1).as_deref() {
        Some("serve") => tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(|e| e.to_string()).and_then(|rt| rt.block_on(serve())),
        _ => Err("usage: open-with serve".into()),
    };
    if let Err(e) = r {
        eprintln!("open-with: {e}");
        std::process::exit(1);
    }
}
