//! The one function of `open.with@1`: for one ce-ref and one viewer, the opener (repo:ce/docs/
//! CE-REF-OPENERS-AND-HOME-2026-11-18.md section 3.2). Pure: the viewer's chain and the openers' liveness come in as
//! functions, so every rule is tested without a network.
//!
//! Filter (a candidate that fails is not ranked, and its name never reaches the viewer): (a) the viewer's chain holds
//! the line's `needs`; (b) its cell answers HealthOp on a replica. Rank, first difference wins: most specific pattern
//! (`code/rust` over `code/*` over `*`), then the line's `rank`. Equal on both is a description error that
//! repo:ce-net-net's `cargo test` refuses. Viewer preference, trust and nearest replica (ranks 1, 3, 5, 6) come after
//! slice 1.
#![forbid(unsafe_code)]

pub use ce_net_net::{Mode as LineMode, Opens};
pub use ce_ref_text::{pattern_matches, specificity, type_of, CeRef, KINDS};
use open_with_api as api;

/// What [`choose`] decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Choice {
    /// the best opener, and how many more the viewer may use
    Target { opener: Opens, others: u8 },
    /// candidates existed, the viewer may use none: the right of the best removed one
    Refused { need: String },
    /// candidates the viewer may use exist, none answers now
    Down { ty: String },
    /// nothing declares this type in this mode
    NoOpener { ty: String },
}

fn line_mode(m: &api::Mode) -> LineMode {
    match m {
        api::Mode::View => LineMode::View,
        api::Mode::Edit => LineMode::Edit,
    }
}

/// Best first: most specific pattern, then rank.
fn ranked<'a>(mut v: Vec<&'a Opens>) -> Vec<&'a Opens> {
    v.sort_by_key(|o| (std::cmp::Reverse(specificity(o.pattern)), std::cmp::Reverse(o.rank)));
    v
}

/// Every line that opens `ty` in `mode`, best first (no filter).
pub fn candidates<'a>(table: &'a [Opens], ty: &str, mode: &api::Mode) -> Vec<&'a Opens> {
    let m = line_mode(mode);
    ranked(table.iter().filter(|o| o.mode == m && pattern_matches(o.pattern, ty)).collect())
}

/// The openers of `ty` this viewer may use, best first, each with whether its cell answers now.
pub fn usable<'a>(table: &'a [Opens], ty: &str, mode: &api::Mode, holds: &dyn Fn(&str) -> bool, live: &dyn Fn(&str, u16) -> bool) -> Vec<(&'a Opens, bool)> {
    candidates(table, ty, mode).into_iter().filter(|o| o.needs.is_empty() || holds(o.needs)).map(|o| (o, live(o.cell, o.major))).collect()
}

pub fn choose(table: &[Opens], ty: &str, mode: &api::Mode, holds: &dyn Fn(&str) -> bool, live: &dyn Fn(&str, u16) -> bool) -> Choice {
    let all = candidates(table, ty, mode);
    if all.is_empty() {
        return Choice::NoOpener { ty: ty.into() };
    }
    let allowed: Vec<&Opens> = all.iter().copied().filter(|o| o.needs.is_empty() || holds(o.needs)).collect();
    if allowed.is_empty() {
        return Choice::Refused { need: all[0].needs.to_string() };
    }
    // The floor (`*`) answers for a type only when nothing more specific is allowed: a viewer refused the record view
    // of a `call:` is told what to ask for, not shown the raw floor of a record.
    let best_spec = specificity(allowed[0].pattern);
    if best_spec == 0 && specificity(all[0].pattern) > 0 {
        return Choice::Refused { need: all[0].needs.to_string() };
    }
    let up: Vec<&Opens> = allowed.into_iter().filter(|o| live(o.cell, o.major)).collect();
    match up.first() {
        Some(best) => Choice::Target { opener: **best, others: (up.len() - 1).min(255) as u8 },
        None => Choice::Down { ty: ty.into() },
    }
}

/// The view's props (JSON text): what the view needs to read the thing as the viewer, nothing it could navigate by.
pub fn props(r: &CeRef, opener: &Opens, sha256: Option<&str>) -> String {
    use serde_json::json;
    let v = match opener.view {
        "ref.view::Text" | "ref.view::Raw" => json!({"ref": r.to_string(), "sha256": sha256}),
        "ref.view::Record" => json!({"kind": r.kind, "id": r.target}),
        "site.cellmap::Cell" => json!({"cell": r.target, "major": r.pin.as_ref().map(|p| p.text())}),
        "site.cellmap::Node" => json!({"node": r.target}),
        "apps.shell::Port" => port_props(r),
        "apps.shell::Outside" => json!({"address": r.target, "site": site_of(&r.target)}),
        _ => json!({"ref": r.to_string()}),
    };
    v.to_string()
}

/// The static sites of ours a port view frames until their real views exist (section 5.4): same origin for huebot;
/// maps.ce-net.com is its own site.
pub fn port_props(r: &CeRef) -> serde_json::Value {
    use serde_json::json;
    match (r.kind.as_str(), r.target.as_str()) {
        ("assembly", "huebot") => json!({"site": "Huebot assembly (3D)", "frame": "/huebot-build/3d/", "step": r.frag.as_deref().and_then(|f| f.strip_prefix("step="))}),
        ("map", t) => json!({"site": format!("Map: {t}"), "frame": "https://maps.ce-net.com/", "place": t}),
        (k, t) => json!({"site": format!("{k}:{t}"), "frame": serde_json::Value::Null}),
    }
}

/// The host of an outside address, for the Outside pane's "you are leaving ce-net" line.
pub fn site_of(address: &str) -> String {
    address.split("://").nth(1).unwrap_or(address).split(['/', '?', '#']).next().unwrap_or("").rsplit('@').next().unwrap_or("").to_string()
}

/// The approval a refused viewer asks for: approve.inbox's request view for that one right.
pub fn ask_ref(need: &str) -> CeRef {
    CeRef::new("view", &format!("approve.inbox@1::Request?need={need}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ce_net_net::opens;

    fn ty(s: &str) -> String {
        type_of(&CeRef::parse(s).unwrap())
    }
    fn everything(_: &str) -> bool {
        true
    }
    fn up(_: &str, _: u16) -> bool {
        true
    }

    #[test]
    fn one_opener_per_type_for_a_viewer_with_every_right() {
        let t = opens();
        let v = |r: &str| match choose(&t, &ty(r), &api::Mode::View, &everything, &up) {
            Choice::Target { opener, .. } => opener.view,
            c => panic!("{r}: {c:?}"),
        };
        assert_eq!(v("repo:apps-shell/crates/apps-shell/src/lib.rs"), "ref.view::Text");
        assert_eq!(v("repo:apps-shell/site/v/search.js"), "ref.view::Text");
        assert_eq!(v("repo:ce/docs/CE-REF-OPENERS-AND-HOME-2026-11-18.md"), "ref.view::Text");
        assert_eq!(v("repo:open-with/tests/fixtures/sample.bin"), "ref.view::Raw");
        assert_eq!(v("call:c-1"), "ref.view::Record");
        assert_eq!(v("promise:deal.promise:x:p-1"), "ref.view::Record");
        assert_eq!(v("person:t-c-daniel-arm"), "ref.view::Record");
        assert_eq!(v("cell:crm.calls@2"), "site.cellmap::Cell");
        assert_eq!(v("node:foreman-ops-1"), "site.cellmap::Node");
        assert_eq!(v("assembly:huebot"), "apps.shell::Port");
        assert_eq!(v("map:twinton"), "apps.shell::Port");
        assert_eq!(v("web:https://example.com/"), "apps.shell::Outside");
    }

    #[test]
    fn a_viewer_with_only_promises_is_refused_a_call_and_told_the_right() {
        let t = opens();
        let holds = |a: &str| a == "crm.promises:read";
        assert_eq!(choose(&t, "call", &api::Mode::View, &holds, &up), Choice::Refused { need: "crm.calls:read".into() });
        assert!(matches!(choose(&t, "promise", &api::Mode::View, &holds, &up), Choice::Target { .. }));
        // a code file with no ref.pin:read: refused Text, never quietly shown the raw floor
        let holds = |a: &str| a == "ref.resolve:read";
        assert_eq!(choose(&t, "code/rust", &api::Mode::View, &holds, &up), Choice::Refused { need: "ref.pin:read".into() });
        // a type only the floor takes still opens in Raw
        assert!(matches!(choose(&t, "*", &api::Mode::View, &holds, &up), Choice::Target { opener: Opens { view: "ref.view::Raw", .. }, .. }));
    }

    #[test]
    fn a_refusal_names_no_opener() {
        let t = opens();
        let holds = |_: &str| false;
        let c = choose(&t, "call", &api::Mode::View, &holds, &up);
        let Choice::Refused { need } = c else { panic!() };
        assert!(!need.contains("::") && !need.contains("ref.view"), "{need}");
        // the ask is an approval ref, not an opener
        assert!(ask_ref(&need).to_string().starts_with("view:approve.inbox@1::Request?need="));
    }

    #[test]
    fn a_stopped_opener_is_not_offered() {
        let t = opens();
        // the record cell down: the floor still shows what the resolver knows (never a dead end)
        let down_crm = |c: &str, _: u16| c != "crm.calls";
        assert!(matches!(choose(&t, "call", &api::Mode::View, &everything, &down_crm), Choice::Target { opener: Opens { view: "ref.view::Raw", .. }, others: 0 }));
        let down_both = |c: &str, _: u16| c != "crm.calls" && c != "ref.resolve";
        assert_eq!(choose(&t, "call", &api::Mode::View, &everything, &down_both), Choice::Down { ty: "call".into() });
        let down_pin = |c: &str, _: u16| c != "ref.pin";
        // Text is down, the floor still answers
        assert!(matches!(choose(&t, "code/rust", &api::Mode::View, &everything, &down_pin), Choice::Target { opener: Opens { view: "ref.view::Raw", .. }, .. }));
    }

    #[test]
    fn most_specific_then_rank() {
        const T: &[Opens] = &[
            Opens { pattern: "code/*", mode: LineMode::View, cell: "a", major: 1, view: "a::Wide", rank: 60, needs: "" },
            Opens { pattern: "code/rust", mode: LineMode::View, cell: "b", major: 1, view: "b::Rust", rank: 5, needs: "" },
            Opens { pattern: "code/rust", mode: LineMode::View, cell: "c", major: 1, view: "c::Rust", rank: 50, needs: "" },
            Opens { pattern: "*", mode: LineMode::View, cell: "d", major: 1, view: "d::Raw", rank: 1, needs: "" },
            Opens { pattern: "code/*", mode: LineMode::Edit, cell: "e", major: 1, view: "e::Editor", rank: 60, needs: "e:write" },
        ];
        match choose(T, "code/rust", &api::Mode::View, &everything, &up) {
            Choice::Target { opener, others } => assert_eq!((opener.view, others), ("c::Rust", 3)),
            c => panic!("{c:?}"),
        }
        match choose(T, "code/js", &api::Mode::View, &everything, &up) {
            Choice::Target { opener, others } => assert_eq!((opener.view, others), ("a::Wide", 1)),
            c => panic!("{c:?}"),
        }
        assert_eq!(choose(T, "doc/pdf", &api::Mode::Edit, &everything, &up), Choice::NoOpener { ty: "doc/pdf".into() });
        assert_eq!(choose(T, "code/js", &api::Mode::Edit, &|_| false, &up), Choice::Refused { need: "e:write".into() });
    }

    #[test]
    fn props_carry_no_address_but_the_ports_and_the_outside() {
        let t = opens();
        let pick = |r: &str| match choose(&t, &ty(r), &api::Mode::View, &everything, &up) {
            Choice::Target { opener, .. } => props(&CeRef::parse(r).unwrap(), &opener, None),
            c => panic!("{c:?}"),
        };
        assert_eq!(pick("call:c-1"), r#"{"id":"c-1","kind":"call"}"#);
        assert!(pick("assembly:huebot#step=14").contains("\"frame\":\"/huebot-build/3d/\""));
        assert!(pick("assembly:huebot#step=14").contains("\"step\":\"14\""));
        assert!(pick("web:https://user@example.com:8443/x").contains("\"site\":\"example.com:8443\""));
        assert!(!pick("repo:x/a.rs").contains("http"));
    }
}
