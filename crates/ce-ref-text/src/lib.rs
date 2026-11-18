//! ce-ref-text: the two written forms of one ce-ref (repo:ce/docs/CE-REF-OPENERS-AND-HOME-2026-11-18.md section 2).
//!
//! - short `kind:target[@pin][#frag]`: prose, chat, commit messages, the address bar ([`CeRef::to_string`]);
//! - full `ce-ref://kind/target[@pin][#frag]`: where an OS or another program must recognise a link ([`CeRef::to_uri`]).
//!
//! Both parse to the same [`CeRef`]. The portable form `ce-ref://repo/<sha256-origin>/<path>` is the full form of kind
//! `repo` and round-trips byte for byte. A ref names a thing, never a place: no host, no port, no checkout path (only
//! `node:` names a machine, because the machine is the thing). Moves into `ce-ref-id` in step 8 of the design.
//!
//! [`type_of`] gives the type string openers match: the kind for record and structural kinds, the media from the file
//! extension for `repo`/`blob`/`media` (`code/rust`, `doc/markdown`, ..., else `*`).
#![forbid(unsafe_code)]

use std::fmt;

/// The kinds a ref may name (ce-ref's 12, live-refs' 9, the record kinds, and `op`, `view`, `map`, `web`).
pub const KINDS: &[&str] = &[
    "repo", "blob", "media", "doc", "person", "promise", "call", "meet", "contract", "cell", "op", "view", "node", "assembly", "part",
    "connection", "map", "app", "skill", "standard", "supplier", "endpoint", "port", "cap", "web", "file",
];

/// Kinds whose type string is the media of the bytes, not the kind.
pub const BYTE_KINDS: &[&str] = &["repo", "blob", "media", "file"];

/// What a pin fixes. A ref with no pin means "the current one".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pin {
    /// a git commit, 7 to 40 lowercase hex
    Commit(String),
    /// `sha256-<64 hex>`: the bytes
    Sha256(String),
    /// a version number: `1`, `1.2`, `1.2.3`
    Version(String),
    /// `asof-<hlc>`: an answer at that time
    AsOf(String),
}

impl Pin {
    /// The pin as written after `@`.
    pub fn text(&self) -> String {
        match self {
            Pin::Commit(c) => c.clone(),
            Pin::Sha256(h) => format!("sha256-{h}"),
            Pin::Version(v) => v.clone(),
            Pin::AsOf(t) => format!("asof-{t}"),
        }
    }

    /// Some(pin) when `s` is a pin; None when the `@` is part of the target.
    pub fn parse(s: &str) -> Option<Pin> {
        let hex = |x: &str| x.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if let Some(h) = s.strip_prefix("sha256-") {
            return (h.len() == 64 && hex(h)).then(|| Pin::Sha256(h.into()));
        }
        if let Some(t) = s.strip_prefix("asof-") {
            return (!t.is_empty() && t.bytes().all(|b| b.is_ascii_alphanumeric() || b"-._".contains(&b))).then(|| Pin::AsOf(t.into()));
        }
        let version = !s.is_empty() && s.len() <= 32 && s.split('.').count() <= 3 && s.split('.').all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
        if version && s.split('.').count() > 1 || version && s.len() <= 3 {
            return Some(Pin::Version(s.into()));
        }
        ((7..=40).contains(&s.len()) && hex(s)).then(|| Pin::Commit(s.into()))
    }
}

/// One ce-ref.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CeRef {
    pub kind: String,
    pub target: String,
    pub pin: Option<Pin>,
    pub frag: Option<String>,
}

/// Why a text is not a ce-ref.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Empty,
    NoKind,
    /// a well formed kind that is not in [`KINDS`]
    UnknownKind(String),
    NoTarget,
    /// spaces, control characters, or a host/port/path where a thing should be named
    Bad(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Empty => write!(f, "an empty ref"),
            ParseError::NoKind => write!(f, "no kind: write kind:target"),
            ParseError::UnknownKind(k) => write!(f, "no kind {k:?} in the kind table"),
            ParseError::NoTarget => write!(f, "no target after the kind"),
            ParseError::Bad(w) => write!(f, "{w}"),
        }
    }
}

impl std::error::Error for ParseError {}

pub const SCHEME: &str = "ce-ref://";

fn kind_ok(k: &str) -> bool {
    !k.is_empty() && k.len() <= 24 && k.as_bytes()[0].is_ascii_lowercase() && k.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == '-' as u8)
}

impl CeRef {
    pub fn new(kind: &str, target: &str) -> CeRef {
        CeRef { kind: kind.into(), target: target.into(), pin: None, frag: None }
    }

    /// Either written form, kind checked against [`KINDS`].
    pub fn parse(s: &str) -> Result<CeRef, ParseError> {
        let r = CeRef::parse_any(s)?;
        if !KINDS.contains(&r.kind.as_str()) {
            return Err(ParseError::UnknownKind(r.kind));
        }
        Ok(r)
    }

    /// Either written form, any well formed kind (the resolver says whether it knows it).
    pub fn parse_any(s: &str) -> Result<CeRef, ParseError> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ParseError::Empty);
        }
        if s.len() > 4096 || s.chars().any(|c| c.is_control() || c == ' ') {
            return Err(ParseError::Bad("a ref has no spaces or control characters and is at most 4096 bytes".into()));
        }
        let (kind, rest) = match s.strip_prefix(SCHEME) {
            Some(full) => full.split_once('/').ok_or(ParseError::NoTarget)?,
            None => s.split_once(':').ok_or(ParseError::NoKind)?,
        };
        if !kind_ok(kind) {
            return Err(if kind.is_empty() { ParseError::NoKind } else { ParseError::Bad(format!("{kind:?} is not a kind (lowercase letters, digits, -)")) });
        }
        // the outside web keeps its address whole: `@` and `#` belong to the URL
        if kind == "web" {
            if !(rest.starts_with("https://") || rest.starts_with("http://")) {
                return Err(ParseError::Bad("a web: ref holds an http(s) address".into()));
            }
            return Ok(CeRef::new("web", rest));
        }
        let (body, frag) = match rest.split_once('#') {
            Some((b, f)) => (b, Some(f.to_string()).filter(|f| !f.is_empty())),
            None => (rest, None),
        };
        let (target, pin) = match body.rsplit_once('@') {
            Some((t, p)) => match Pin::parse(p) {
                Some(pin) if !t.is_empty() => (t, Some(pin)),
                _ => (body, None),
            },
            None => (body, None),
        };
        if target.is_empty() {
            return Err(ParseError::NoTarget);
        }
        if target.contains("://") {
            return Err(ParseError::Bad("a ref names a thing, never a place: wrap an address as web:<url>".into()));
        }
        Ok(CeRef { kind: kind.into(), target: target.into(), pin, frag })
    }

    fn tail(&self) -> String {
        let mut s = self.target.clone();
        if let Some(p) = &self.pin {
            s.push('@');
            s.push_str(&p.text());
        }
        if let Some(f) = &self.frag {
            s.push('#');
            s.push_str(f);
        }
        s
    }

    /// The full form, for the clipboard, a deep link, a QR code or mail.
    pub fn to_uri(&self) -> String {
        format!("{SCHEME}{}/{}", self.kind, self.tail())
    }

    /// The same thing at any pin and fragment (what an opener is chosen for).
    pub fn unpinned(&self) -> CeRef {
        CeRef::new(&self.kind, &self.target)
    }

    /// The last path segment's extension, lowercased (`rs` of `repo:x/src/lib.rs`).
    pub fn extension(&self) -> Option<String> {
        let last = self.target.rsplit('/').next()?;
        let (stem, ext) = last.rsplit_once('.')?;
        (!stem.is_empty() && !ext.is_empty() && ext.len() <= 8).then(|| ext.to_ascii_lowercase())
    }
}

/// The short form.
impl fmt::Display for CeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind, self.tail())
    }
}

impl std::str::FromStr for CeRef {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<CeRef, ParseError> {
        CeRef::parse(s)
    }
}

/// The media of a file by its extension; `*` when none is known.
pub fn media_of_ext(ext: &str) -> &'static str {
    match ext {
        "rs" => "code/rust",
        "js" | "mjs" | "cjs" => "code/js",
        "ts" | "tsx" => "code/ts",
        "py" => "code/python",
        "go" => "code/go",
        "swift" => "code/swift",
        "kt" => "code/kotlin",
        "sh" | "bash" => "code/shell",
        "html" | "htm" => "code/html",
        "css" => "code/css",
        "c" | "h" => "code/c",
        "toml" => "text/toml",
        "json" => "text/json",
        "yaml" | "yml" => "text/yaml",
        "txt" | "log" => "text/plain",
        "csv" => "table/csv",
        "md" | "markdown" => "doc/markdown",
        "pdf" => "doc/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg",
        "webp" => "image/webp",
        "glb" => "3d/glb",
        "stl" => "3d/stl",
        _ => "*",
    }
}

/// The type string openers match (design 2.3): the media for byte kinds, else the kind.
pub fn type_of(r: &CeRef) -> String {
    if BYTE_KINDS.contains(&r.kind.as_str()) {
        let name = r.target.rsplit('/').next().unwrap_or("");
        // a file with no extension that is plainly text by name
        if matches!(name, "README" | "LICENSE" | "Makefile" | "Dockerfile" | "NEXT") {
            return "text/plain".into();
        }
        return r.extension().map(|e| media_of_ext(&e).to_string()).unwrap_or_else(|| "*".into());
    }
    r.kind.clone()
}

/// Does an opener's pattern take this type? `code/rust`, `code/*`, `*`.
pub fn pattern_matches(pattern: &str, ty: &str) -> bool {
    if pattern == "*" || pattern == ty {
        return true;
    }
    match pattern.strip_suffix("/*") {
        Some(head) => ty.strip_prefix(head).is_some_and(|t| t.starts_with('/')),
        None => false,
    }
}

/// How specific a pattern is: exact 2, `x/*` 1, `*` 0 (design 3.2 rank 2).
pub fn specificity(pattern: &str) -> u8 {
    if pattern == "*" {
        0
    } else if pattern.ends_with("/*") {
        1
    } else {
        2
    }
}

/// The ranges of `text` that parse as a ref of a known kind (`host.ref.find`): (start, end, ref).
pub fn find(text: &str) -> Vec<(usize, usize, CeRef)> {
    let mut out = Vec::new();
    let mut i = 0;
    let b = text.as_bytes();
    while i < b.len() {
        let start_ok = i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'-' || b[i - 1] == b'/');
        if start_ok && b[i].is_ascii_lowercase() {
            let end = text[i..].find(|c: char| c.is_whitespace() || c == '<' || c == '>' || c == '"' || c == '`' || c == ')' || c == ']').map(|n| i + n).unwrap_or(b.len());
            let mut cand = &text[i..end];
            cand = cand.trim_end_matches(['.', ',', ';', ':', '!', '?', '\'']);
            if cand.contains(':') {
                if let Ok(r) = CeRef::parse(cand) {
                    out.push((i, i + cand.len(), r));
                    i += cand.len();
                    continue;
                }
            }
        }
        i += text[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt(s: &str) {
        let r = CeRef::parse(s).unwrap_or_else(|e| panic!("{s}: {e}"));
        let short = r.to_string();
        let full = r.to_uri();
        assert_eq!(CeRef::parse(&short).unwrap(), r, "short of {s}");
        assert_eq!(CeRef::parse(&full).unwrap(), r, "full of {s}");
        assert_eq!(CeRef::parse(&full).unwrap().to_uri(), full);
    }

    #[test]
    fn both_forms_are_one_ref() {
        let a = CeRef::parse("repo:apps-shell/site/v/search.js@9710bcd#L31").unwrap();
        let b = CeRef::parse("ce-ref://repo/apps-shell/site/v/search.js@9710bcd#L31").unwrap();
        assert_eq!(a, b);
        assert_eq!(a.kind, "repo");
        assert_eq!(a.target, "apps-shell/site/v/search.js");
        assert_eq!(a.pin, Some(Pin::Commit("9710bcd".into())));
        assert_eq!(a.frag.as_deref(), Some("L31"));
        assert_eq!(a.to_string(), "repo:apps-shell/site/v/search.js@9710bcd#L31");
        assert_eq!(a.to_uri(), "ce-ref://repo/apps-shell/site/v/search.js@9710bcd#L31");
    }

    #[test]
    fn the_portable_form_stays_valid_byte_for_byte() {
        let origin = "4f1c0e9b2a7d3c5e6f8091a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6";
        let s = format!("ce-ref://repo/{origin}/crates/x/src/lib.rs");
        let r = CeRef::parse(&s).unwrap();
        assert_eq!(r.kind, "repo");
        assert_eq!(r.target, format!("{origin}/crates/x/src/lib.rs"));
        assert_eq!(r.to_uri(), s);
        assert_eq!(r.to_string(), format!("repo:{origin}/crates/x/src/lib.rs"));
    }

    #[test]
    fn round_trips() {
        for s in [
            "assembly:huebot",
            "assembly:huebot#step=14",
            "map:twinton/floor-2#device=printer-3",
            "person:t-c-daniel-arm",
            "call:c-2026-11-18-0930#t=132",
            "promise:deal.promise:huebot:p-88",
            "cell:crm.calls@1",
            "node:foreman-ops-1",
            "op:crm.calls@1/Today?day=2026-11-18",
            "view:approve.inbox@1::Pending?about=call:c-1",
            "media:x@sha256-4f1c0e9b2a7d3c5e6f8091a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6",
            "doc:d-7c21#h=terms",
            "repo:ce/docs/CE-REF-OPENERS-AND-HOME-2026-11-18.md",
            "op:crm.calls/Today@asof-1794990000.12",
            "cap:id-91ab",
        ] {
            rt(s);
        }
    }

    #[test]
    fn pins() {
        assert_eq!(CeRef::parse("cell:crm.calls@1").unwrap().pin, Some(Pin::Version("1".into())));
        assert_eq!(CeRef::parse("cell:crm.calls@1").unwrap().target, "crm.calls");
        assert_eq!(CeRef::parse("app:vivi@1.2.3").unwrap().pin, Some(Pin::Version("1.2.3".into())));
        // an @ that is no pin stays in the target
        let r = CeRef::parse("op:crm.calls@1/Today").unwrap();
        assert_eq!((r.target.as_str(), r.pin.clone()), ("crm.calls@1/Today", None));
        let r = CeRef::parse("person:someone@example").unwrap();
        assert_eq!((r.target.as_str(), r.pin.clone()), ("someone@example", None));
    }

    #[test]
    fn web_keeps_its_address_whole() {
        let r = CeRef::parse("web:https://user@example.com/x?a=1#top").unwrap();
        assert_eq!(r.target, "https://user@example.com/x?a=1#top");
        assert_eq!((&r.pin, &r.frag), (&None, &None));
        assert_eq!(r.to_string(), "web:https://user@example.com/x?a=1#top");
        assert_eq!(CeRef::parse(&r.to_uri()).unwrap(), r);
        assert!(CeRef::parse("web:example.com").is_err());
    }

    #[test]
    fn refusals() {
        assert_eq!(CeRef::parse(""), Err(ParseError::Empty));
        assert_eq!(CeRef::parse("huebot"), Err(ParseError::NoKind));
        assert_eq!(CeRef::parse("frob:x"), Err(ParseError::UnknownKind("frob".into())));
        assert!(CeRef::parse_any("frob:x").is_ok());
        assert_eq!(CeRef::parse("repo:"), Err(ParseError::NoTarget));
        assert!(CeRef::parse("repo:a b").is_err());
        assert!(CeRef::parse("Repo:x").is_err());
        assert!(CeRef::parse("repo:https://github.com/x").is_err(), "a place is not a thing");
    }

    #[test]
    fn types() {
        let t = |s: &str| type_of(&CeRef::parse(s).unwrap());
        assert_eq!(t("repo:apps-shell/crates/apps-shell/src/lib.rs"), "code/rust");
        assert_eq!(t("repo:apps-shell/site/v/search.js@9710bcd"), "code/js");
        assert_eq!(t("repo:ce/docs/X.md"), "doc/markdown");
        assert_eq!(t("repo:x/notes.txt"), "text/plain");
        assert_eq!(t("repo:x/a.png"), "image/png");
        assert_eq!(t("repo:open-with/tests/fixtures/sample.bin"), "*");
        assert_eq!(t("repo:x/README"), "text/plain");
        assert_eq!(t("call:c-1"), "call");
        assert_eq!(t("assembly:huebot#step=3"), "assembly");
        assert_eq!(t("web:https://example.com/a.rs"), "web");
    }

    #[test]
    fn patterns() {
        assert!(pattern_matches("code/*", "code/rust"));
        assert!(pattern_matches("*", "call"));
        assert!(pattern_matches("call", "call"));
        assert!(!pattern_matches("code/*", "code"));
        assert!(!pattern_matches("code/*", "codex/rust"));
        assert!(!pattern_matches("doc/markdown", "doc/pdf"));
        assert!(specificity("code/rust") > specificity("code/*") && specificity("code/*") > specificity("*"));
    }

    #[test]
    fn find_in_text() {
        let f = find("Brief for call:c-1, see person:t-c-daniel-arm and repo:ce/docs/X.md. Not a ref: http://x or frob:y.");
        let refs: Vec<String> = f.iter().map(|(_, _, r)| r.to_string()).collect();
        assert_eq!(refs, vec!["call:c-1", "person:t-c-daniel-arm", "repo:ce/docs/X.md"]);
    }
}
