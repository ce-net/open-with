//! `open.with@1`: which cell's view opens a ce-ref, for this viewer (repo:ce/docs/CE-REF-OPENERS-AND-HOME-2026-11-18.md
//! section 3.2). The answer is one target, a named refusal, or a named absence; never a guess between equal openers and
//! never the name of an opener the viewer may not use.
//!
//! Slice 1: `Open`, `Handlers`, `HealthOp` are real; `Peek`, `Prefer`, `Changes` answer `NotYet`.
#![forbid(unsafe_code)]

pub use ce_ref_text;

ce_wire::api! {
    api OpenWith = "open.with", version 1;
    types {
        /// Health contract: these three fields, in this order, in every cell.
        pub struct Health { pub ok: bool, pub version: String, pub uptime_s: u64 }
        /// What a pin fixes (ce-ref-text `Pin`).
        pub enum Pin { Commit(String), Sha256(String), Version(String), AsOf(String) }
        /// One ce-ref (ce-ref-text `CeRef`): a thing's identity, never its place.
        pub struct CeRef { pub kind: String, pub target: String, pub pin: Option<Pin>, pub frag: Option<String> }
        pub enum Mode { View, Edit }
        pub enum Device { Phone, Desk, Wall }
        pub struct OpenReq { pub r: CeRef, pub mode: Mode, pub device: Device }
        /// `Target`: mount `view` of `cell@major` with `props` (JSON text); `r` is the ref resolved (pinned when the
        /// resolver gave a content hash), `ty` its type string, `others` how many further openers this viewer may use.
        /// `Refused`: the thing exists, the viewer lacks `need`; `ask` is the approval to ask for it. No opener is named.
        /// `NoOpener`: nothing declares this type. `Unknown`: the resolver's own words (no such thing, or no opener of
        /// it answers now).
        pub enum Opened {
            Target { cell: String, major: u16, view: String, props: String, r: CeRef, ty: String, others: u8 },
            Refused { need: String, ask: CeRef },
            NoOpener { ty: String },
            Unknown { reason: String },
        }
        pub struct HandlersReq { pub ty: String, pub mode: Mode }
        /// One opener this viewer may use for a type, best first.
        pub struct Handler { pub cell: String, pub major: u16, pub view: String, pub pattern: String, pub rank: u16, pub live: bool }
        pub struct PeekReq { pub refs: Vec<CeRef> }
        pub struct PreferReq { pub ty: String, pub mode: Mode, pub handler: Option<String> }
        pub struct ChangesReq { pub after: u64, pub wait_ms: u32 }
        pub enum OpenError { NotYet(String), Invalid(String) }
    }
    ops {
        1 => HealthOp(()) -> Health, needs "open.with:health";
        2 => Open(OpenReq) -> Opened, needs "open.with:use";
        3 => Handlers(HandlersReq) -> Vec<Handler>, needs "open.with:use";
        4 => Peek(PeekReq) -> Result<Vec<String>, OpenError>, needs "open.with:use";
        5 => Prefer(PreferReq) -> Result<(), OpenError>, needs "open.with:use";
        6 => Changes(ChangesReq) -> Result<u64, OpenError>, needs "open.with:use";
    }
}

impl From<ce_ref_text::CeRef> for CeRef {
    fn from(r: ce_ref_text::CeRef) -> CeRef {
        CeRef {
            kind: r.kind,
            target: r.target,
            pin: r.pin.map(|p| match p {
                ce_ref_text::Pin::Commit(c) => Pin::Commit(c),
                ce_ref_text::Pin::Sha256(h) => Pin::Sha256(h),
                ce_ref_text::Pin::Version(v) => Pin::Version(v),
                ce_ref_text::Pin::AsOf(t) => Pin::AsOf(t),
            }),
            frag: r.frag,
        }
    }
}

impl From<CeRef> for ce_ref_text::CeRef {
    fn from(r: CeRef) -> ce_ref_text::CeRef {
        ce_ref_text::CeRef {
            kind: r.kind,
            target: r.target,
            pin: r.pin.map(|p| match p {
                Pin::Commit(c) => ce_ref_text::Pin::Commit(c),
                Pin::Sha256(h) => ce_ref_text::Pin::Sha256(h),
                Pin::Version(v) => ce_ref_text::Pin::Version(v),
                Pin::AsOf(t) => ce_ref_text::Pin::AsOf(t),
            }),
            frag: r.frag,
        }
    }
}

impl CeRef {
    /// Either written form, kind checked.
    pub fn parse(s: &str) -> Result<CeRef, ce_ref_text::ParseError> {
        ce_ref_text::CeRef::parse(s).map(CeRef::from)
    }
    pub fn short(&self) -> String {
        ce_ref_text::CeRef::from(self.clone()).to_string()
    }
    pub fn uri(&self) -> String {
        ce_ref_text::CeRef::from(self.clone()).to_uri()
    }
}
