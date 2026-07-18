//! PASS 36 — THE RECALL CENSUS, re-cut to the two-axis law (LINTER.md, "COMPLETION PASS 36",
//! owner ruling 2026-07-18): LEARNING and FIRING are separate axes, and only the second may ever
//! decline.
//!
//!   * KNOWLEDGE AXIS — hard 100%, no exceptions. After train, EVERY manifest fact is present in
//!     the machine's knowledge for its language: the subject is KNOWN, with the docs' own
//!     governing prose and its negative polarity. Blockless sections, abstain-traps, member
//!     shapes, mixed-language pages — all of it. A ledger row is NEVER a reason to not-know.
//!     Observed out-of-process via `lint_query kind=web arg=<subject> language=<lang>`
//!     (see [`Census::known`] for the exact probe and the src TODO contract it pins).
//!   * ENFORCEMENT AXIS — tiered, honest. Every KNOWN fact fires at some tier (proven rule /
//!     graded LOW), or its conservation-ledger row (`lint_query kind=rules` → `withheld[]`)
//!     NAMES why a detector is unsafe. "Silently lost" is the failure class this census forbids;
//!     the ratchet's unit of progress is pushing a known fact UP a tier, never learning less.
//!
//! A machine-readable FACT MANIFEST is ground truth. The fixture site is GENERATED from it in two
//! renders — CLEAN (minimal honest HTML) and MESSY (recurring chrome, furniture sections, member
//! pages, mixed-language page) — served from localhost and trained through the REAL binary
//! exactly as a user trains it. Both tests print the two accounting lines
//! (`knowledge K/N` and `enforcement: proven X + graded Y + quiet-with-reason Z + silent S`)
//! and assert knowledge K == N and silent == 0. The CLEAN render additionally asserts every
//! fact's exact expected enforcement; the MESSY render is the accounting instrument — its
//! divergence from CLEAN is the punch list for dissolving the interim HTML heuristics ("the page
//! should be READ, not split").
//!
//! Tests are hermetic exactly as `ai_linter_behaviors.rs`: caches and the learned-source registry
//! are redirected into the test's temp dir (`HELPERS_LINT_MODELS`, `HOME`), and the network is
//! used only to read the test's own localhost docs during the one training step.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};

// ── Hermetic harness (the ai_linter_behaviors.rs TestProject, verbatim) ───────────────────────

/// A throwaway project directory plus the isolated cache/home dirs a hermetic lint run needs.
struct TestProject {
    root: PathBuf,
}

impl TestProject {
    /// Create an empty git project under a unique temp dir.
    fn new(name: &str) -> TestProject {
        let root = std::env::temp_dir().join(format!("ai-lint-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp project dir");
        let ok = Command::new("git")
            .args(["init", "-q"])
            .current_dir(&root)
            .status()
            .expect("git available")
            .success();
        assert!(ok, "git init failed");
        TestProject { root }
    }

    /// Write a file (creating parents) relative to the project root.
    fn write(&self, rel: &str, content: &str) -> &Self {
        let path = self.root.join(rel);
        fs::create_dir_all(path.parent().expect("has parent")).expect("mkdir");
        fs::write(path, content).expect("write fixture file");
        self
    }

    /// Run `helpers-native call lint` on this project and return the rendered verdict text.
    /// `offline` disables network learning so the run is deterministic.
    fn lint(&self, offline: bool) -> String {
        self.call("lint", &format!(r#"{{"root":{:?}}}"#, self.root.to_string_lossy()), offline)
    }

    /// Run any tool of the binary against this project; returns the tool's rendered text.
    fn call(&self, tool: &str, request: &str, offline: bool) -> String {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_helpers-native"));
        cmd.args(["call", tool])
            .current_dir(&self.root)
            .env("HELPERS_LINT_MODELS", self.root.join(".test-models"))
            .env("HOME", self.root.join(".test-home"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if offline {
            cmd.env("HELPERS_LINT_OFFLINE", "1");
        }
        let mut child = cmd.spawn().expect("binary runs");
        child
            .stdin
            .as_mut()
            .expect("stdin piped")
            .write_all(request.as_bytes())
            .expect("request written");
        let out = child.wait_with_output().expect("binary exits");
        assert!(
            out.status.success(),
            "`call {tool}` must exit cleanly.\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
        let raw = String::from_utf8_lossy(&out.stdout).into_owned();
        let json: serde_json::Value = serde_json::from_str(raw.trim())
            .unwrap_or_else(|e| panic!("tool output is protocol JSON ({e}): {raw}"));
        json["content"][0]["text"].as_str().expect("text content").to_string()
    }
}

/// True when `rule` is flagged within `file`'s section of the verdict.
fn flagged_in(verdict: &str, file: &str, rule: &str) -> bool {
    verdict
        .lines()
        .skip_while(|l| !l.contains(file))
        .take_while(|l| !l.trim().is_empty())
        .any(|l| l.contains(rule))
}

// ── The fact manifest — ground truth the site is generated FROM ───────────────────────────────

/// The ENFORCEMENT expectation the manifest asserts for one fact (the knowledge axis has no
/// per-fact expectation — it is 100% for every fact except `Nothing`, by owner law).
/// `Nothing` is the tutorial-negative contract: a sample program states no fact — nothing may
/// enter knowledge under its name, nothing may mint, nothing may fire, and the fact is counted
/// outside both axes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Expect {
    /// Fires at the PROVEN tier: the bad file flags the subject with the docs' own forbidding
    /// words, the clean file stays silent, and the web node is PROVEN.
    FiresProven,
    /// Fires at SOME tier — proven OR graded LOW, either accepted — and the finding cites the
    /// docs' own forbidding sentence (the blockless `flare` ratchet: known 100%, tier may vary).
    FiresAnyTier,
    /// Fires by the docs' DEMONSTRATED SHAPE (owner rulings 2026-07-18, second + third): the
    /// bad file's planted NOVEL bad-shape instance flags, while the clean file — carrying the
    /// novel clean twin AND the docs' own good form — stays totally silent (the abstain-trap's
    /// rule IS the trailing separator, never the bare token, never the docs' verbatim bytes).
    FiresShape,
    /// Fires ONLY by the author's DOTTED typography (the member-shape contract): the bad file's
    /// planted `Owner.subject` instance flags, and a bare token in clean code NEVER does.
    FiresDotted,
    /// Outside both axes: nothing minted, nothing fired, nothing known under the subject's name.
    Nothing,
}

/// One authored documentation fact: what a page STATES, where it states it, and the honest
/// terminal state the census asserts for it. The fixture site (both renders) is generated from
/// these records — the manifest is the single source of truth the census measures against.
#[derive(Clone, Copy, Debug)]
struct Fact {
    /// Stable census id, printed in the accounting and the failure lists.
    id: &'static str,
    /// The language the fact governs (also the planted files' extension: nothing claims a fake
    /// extension, so the extension IS the language name — the learned-extension fallback).
    lang: &'static str,
    /// Site path of the page that states the fact (multiple facts may share a page: mixed).
    page: &'static str,
    /// Page/fact shape: prohibition | blockless-prohibition | deprecation | abstain-trap |
    /// member-shape | tutorial-negative | cross-language.
    kind: &'static str,
    /// The construct token the fact governs — the join key into verdict lines, ledger ids, and
    /// the knowledge probe (`lint_query kind=web`).
    subject: &'static str,
    /// The docs' own forbidding sentence, rendered verbatim on the page.
    forbid_sentence: &'static str,
    /// Code that BREAKS the fact as the PAGE demonstrates it (empty: nothing to render).
    bad_code: &'static str,
    /// Code that OBEYS the fact as the PAGE demonstrates it (empty: nothing to render).
    good_code: &'static str,
    /// Override for the code planted in `bad.<lang>` when it must differ from the page's own
    /// example (`None` → `bad_code`). The member fact plants the DOTTED shape here: the page
    /// demonstrates the bare shape, but only the parent-typography form may ever fire.
    plant_bad: Option<&'static str>,
    /// Override for the code planted in `clean.<lang>` (`None` → `good_code`). The member fact
    /// plants a BARE `grip` token here — the shape that must never flag in clean code.
    plant_good: Option<&'static str>,
    /// The honest expected ENFORCEMENT today (pushing a fact up a tier is the ratchet).
    expected: Expect,
}

impl Fact {
    /// The code actually planted in `bad.<lang>` for this fact.
    fn planted_bad(&self) -> &'static str {
        self.plant_bad.unwrap_or(self.bad_code)
    }
    /// The code actually planted in `clean.<lang>` for this fact.
    fn planted_good(&self) -> &'static str {
        self.plant_good.unwrap_or(self.good_code)
    }
}

/// The census manifest: every kind the PASS 36 contract lists, across TWO fake languages —
/// including the cross-language pair (one page each) and the mixed-language page both crawls
/// read. Enforcement expectations per the 2026-07-18 owner rulings (all three): the six
/// reference prohibitions fire PROVEN; blockless `flare` fires at SOME tier (proven or graded
/// LOW); abstain-trap `packvex` fires by its DEMONSTRATED SHAPE on a novel instance; member
/// `grip` fires by the author's DOTTED typography; the tutorial states nothing. Perfect docs
/// leave nothing quiet: named-quiet is asserted to ZERO on this manifest, both renders. The
/// KNOWLEDGE axis is not per-fact: every fact except the tutorial must be known, 100%.
fn manifest() -> Vec<Fact> {
    vec![
        Fact {
            id: "vex-no-zapcall",
            lang: "vex",
            page: "/vex/zapcall",
            kind: "prohibition",
            subject: "zapcall",
            forbid_sentence:
                "Never use the zapcall statement anywhere; it is deprecated and will be removed.",
            bad_code: "zapcall(payload)",
            good_code: "safecall(payload)",
            plant_bad: None,
            plant_good: None,
            expected: Expect::FiresProven,
        },
        Fact {
            id: "vex-no-flare-blockless",
            lang: "vex",
            page: "/vex/flare",
            kind: "blockless-prohibition",
            subject: "flare",
            forbid_sentence:
                "Never use the flare statement anywhere; it is dangerous and will be removed.",
            bad_code: "flare now",
            good_code: "",
            plant_bad: None,
            plant_good: None,
            expected: Expect::FiresAnyTier,
        },
        Fact {
            id: "vex-no-wobble-deprecated",
            lang: "vex",
            page: "/vex/wobble",
            kind: "deprecation",
            subject: "wobble",
            forbid_sentence:
                "Deprecated: never use the wobble form in new code; it is deprecated and will be removed.",
            bad_code: "wobble mode",
            good_code: "bobble mode",
            plant_bad: None,
            plant_good: None,
            expected: Expect::FiresProven,
        },
        Fact {
            id: "vex-packvex-abstain-trap",
            lang: "vex",
            page: "/vex/errors/packvex",
            kind: "abstain-trap",
            subject: "packvex",
            forbid_sentence:
                "Never write a trailing separator in a packvex literal; the string cannot be parsed and the program is rejected.",
            bad_code: "packvex(\"[1,2,]\")",
            good_code: "packvex(\"[1,2]\")",
            // NOVEL-INSTANCE probes (third ruling): the planted violation is a shape the docs
            // never spelled verbatim — a rule that only re-recognizes the docs' own bytes
            // fails; the clean file carries the novel clean twin plus the docs' own good form.
            plant_bad: Some("packvex(\"[7,8,9,]\")"),
            plant_good: Some("packvex(\"[7,8,9]\")\npackvex(\"[1,2]\")"),
            // bad/good differ only inside a string literal — exactly the contrast the
            // demonstrated-shape compile reads: the detector is the anchored diff (`,]` in the
            // packvex context), so the shape FIRES and nothing is quiet.
            expected: Expect::FiresShape,
        },
        Fact {
            id: "vex-no-grip-member",
            lang: "vex",
            page: "/vex/reference/objects/Gadget/grip",
            kind: "member-shape",
            subject: "grip",
            forbid_sentence:
                "Never use the grip member anywhere; it is deprecated and will be removed.",
            // The member PAGE demonstrates the bare shape; the parent page's typography is
            // dotted. Planted files probe the ruling's exact boundary: the dotted shape in the
            // bad file MUST fire (a novel argument — the parent spells `handle`); the bare
            // token in the clean file must NEVER flag.
            bad_code: "grip(handle)",
            good_code: "clamp(handle)",
            plant_bad: Some("Gadget.grip(lever)"),
            plant_good: Some("grip = clamp(handle)"),
            expected: Expect::FiresDotted,
        },
        Fact {
            id: "vex-greet-tutorial",
            lang: "vex",
            page: "/vex/getting-started",
            kind: "tutorial-negative",
            subject: "greet",
            forbid_sentence: "",
            bad_code: "",
            good_code: "greet(\"world\")",
            plant_bad: None,
            plant_good: None,
            expected: Expect::Nothing,
        },
        Fact {
            id: "vex-no-glim-pair",
            lang: "vex",
            page: "/vex/glim",
            kind: "cross-language",
            subject: "glim",
            forbid_sentence:
                "Never use the glim statement anywhere; it is deprecated and will be removed.",
            bad_code: "glim spark",
            good_code: "glow spark",
            plant_bad: None,
            plant_good: None,
            expected: Expect::FiresProven,
        },
        Fact {
            id: "zim-no-zaploop-pair",
            lang: "zim",
            page: "/zim/zaploop",
            kind: "cross-language",
            subject: "zaploop",
            forbid_sentence:
                "Never use the zaploop statement anywhere; it is deprecated and will be removed.",
            bad_code: "zaploop(tick)",
            good_code: "zapstep(tick)",
            plant_bad: None,
            plant_good: None,
            expected: Expect::FiresProven,
        },
        Fact {
            id: "vex-no-mixvex-mixed-page",
            lang: "vex",
            page: "/shared/mixed",
            kind: "cross-language",
            subject: "mixvex",
            forbid_sentence:
                "In vex code, never use the mixvex statement anywhere; it is deprecated and will be removed.",
            bad_code: "mixvex go",
            good_code: "mixsafe go",
            plant_bad: None,
            plant_good: None,
            expected: Expect::FiresProven,
        },
        Fact {
            id: "zim-no-mixzim-mixed-page",
            lang: "zim",
            page: "/shared/mixed",
            kind: "cross-language",
            subject: "mixzim",
            forbid_sentence:
                "In zim code, never use the mixzim statement anywhere; it is deprecated and will be removed.",
            bad_code: "mixzim run",
            good_code: "mixcalm run",
            plant_bad: None,
            plant_good: None,
            expected: Expect::FiresProven,
        },
    ]
}

/// The docs' own forbidding words the FIRES classification requires in the verdict: the
/// `use the <subject> <noun>` clause of the fact's sentence — distinctive enough to prove the
/// finding cites the SOURCE sentence, short enough to survive rendering. `None` when the
/// sentence carries no such clause (the abstain trap), in which case the words check is waived.
fn forbid_words(f: &Fact) -> Option<String> {
    let needle = format!("use the {} ", f.subject);
    let at = f.forbid_sentence.find(&needle)?;
    let rest = &f.forbid_sentence[at..];
    let end = rest
        .match_indices(' ')
        .nth(3) // "use" + "the" + subject + the noun that follows
        .map(|(i, _)| i)
        .unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

// ── Site generation — CLEAN and MESSY renders from the SAME manifest ──────────────────────────

/// Recurring site chrome for the MESSY render: NOT inside `<nav>`/`<footer>` tags (plain divs
/// with ids and css-ish text), repeated verbatim on EVERY page so the per-host chrome keying has
/// the page support it needs (the site serves well over 8 pages).
const CHROME_TOP: &str = "<div id=\"menu_click_menu--site\">SITE reference \
     .button{align-items:center;border:1px solid}</div>\
     <div id=\"breadcrumb_click--2\">Reference</div>";
const CHROME_BOTTOM: &str = "<div id=\"footer--link\">Site map</div>";

/// Furniture sections for the MESSY render: `<h2 id>` slugs from `NON_GOVERNING_ANCHORS`, whose
/// prose must never mint or surface in findings.
const FURNITURE: &str = "<h2 id=\"specifications\">Specifications</h2>\
     <p>Runtime Living Standard, section 4.</p>\
     <h2 id=\"see_also\">See also</h2>\
     <p>Related calls and the bus overview.</p>";

/// The `<h1>` a fact's page opens with, by page shape.
fn page_h1(f: &Fact) -> String {
    match f.kind {
        "deprecation" => format!("The {} form", f.subject),
        "abstain-trap" => format!("SyntaxError: {} literal", f.subject),
        "tutorial-negative" => "Getting started".to_string(),
        _ => format!("{}()", f.subject),
    }
}

/// One fact rendered as its page SECTION: anchored heading (the slug carries the subject so the
/// minted rule id does too), the forbidding sentence, and the example blocks its kind provides —
/// a bad/good pair for reference shapes, punctuation-only pair for the abstain trap, prose alone
/// for the blockless shape, and a neutral sample program for the tutorial.
fn section_html(f: &Fact) -> String {
    match f.kind {
        "blockless-prohibition" => format!(
            "<h2 id=\"never_use_{s}\">Never use {s}!</h2><p>{p}</p>",
            s = f.subject,
            p = f.forbid_sentence
        ),
        // The anchored heading keeps the page's shared lead paragraph OUT of this section:
        // welded lead + `Deprecated:` prose dilutes the register and nothing is minted
        // (the proven diversity-test shape, tests/ai_linter_behaviors.rs wobble page).
        "deprecation" => format!(
            "<h2 id=\"{s}_deprecated\">Deprecated</h2><p>{p}</p><pre>{b}</pre>\
             <p>Use the bobble form instead; this is the correct form:</p><pre>{g}</pre>",
            s = f.subject,
            p = f.forbid_sentence,
            b = f.bad_code,
            g = f.good_code
        ),
        "abstain-trap" => format!(
            "<h2 id=\"{s}_trailing_separator\">Trailing separators</h2><p>{p}</p>\
             <pre>{b}</pre>\
             <p>Use the plain form instead; this is the correct form:</p><pre>{g}</pre>",
            s = f.subject,
            p = f.forbid_sentence,
            b = f.bad_code,
            g = f.good_code
        ),
        "tutorial-negative" => format!(
            "<p>A first program prints a greeting and finishes cleanly.</p>\
             <pre>begin\n  {g}\n  finish\nend</pre>\
             <p>Run it with the runner and read the output.</p>",
            g = f.good_code
        ),
        // prohibition | member-shape | cross-language: the anchored reference shape.
        _ => format!(
            "<h2 id=\"never_use_{s}\">Never use {s}!</h2><p>{p}</p>\
             <pre>{b}</pre>\
             <p>Use a structured call instead; this is the correct form:</p><pre>{g}</pre>",
            s = f.subject,
            p = f.forbid_sentence,
            b = f.bad_code,
            g = f.good_code
        ),
    }
}

/// Generate the whole site from the manifest: one entry per path. Facts sharing a page (the
/// mixed-language page) contribute consecutive sections under one `<h1>`; the member fact's
/// OWNER page carries the parent's dotted typography (what the member veto reads); each
/// language gets an index page linking every page its crawl must reach (shared pages linked
/// from BOTH indexes), and a site-root index links both language indexes — the page both
/// root-seeded crawls start from. `messy` wraps every page in the recurring chrome and
/// furniture.
fn build_site(facts: &[Fact], messy: bool) -> BTreeMap<String, String> {
    // Page bodies: group the facts' sections by path, first fact's h1 + a subject-naming lead.
    let mut inner: BTreeMap<String, String> = BTreeMap::new();
    for f in facts {
        let body = inner.entry(f.page.to_string()).or_insert_with(|| {
            format!(
                "<h1>{}</h1><p>The {} call is part of the classic interface and appears \
                 throughout older programs.</p>",
                page_h1(f),
                f.subject
            )
        });
        body.push_str(&section_html(f));
    }
    // The member fact's PARENT page: the owner's own typography is dotted — the evidence the
    // member veto cites when it withholds the bare shape a member page proposes.
    inner.insert(
        "/vex/reference/objects/Gadget".to_string(),
        "<h1>Gadget</h1>\
         <p>The Gadget element exposes its members with dotted typography.</p>\
         <pre>Gadget.grip(handle)\nGadget.clamp(handle)</pre>"
            .to_string(),
    );
    // Per-language index pages: link every page of that language plus the shared pages.
    for lang in ["vex", "zim"] {
        let links: String = inner
            .keys()
            .filter(|p| p.starts_with(&format!("/{lang}/")) || p.starts_with("/shared/"))
            .map(|p| format!("<a href=\"{p}\">{}</a> ", p.rsplit('/').next().unwrap_or(p)))
            .collect();
        inner.insert(
            format!("/{lang}/"),
            format!("<h1>{lang} documentation</h1><p>The reference for the {lang} language.</p>{links}"),
        );
    }
    // Site-root index: both crawls seed HERE (the whole site in scope for both languages),
    // reaching every page through the per-language indexes it links.
    inner.insert(
        "/".to_string(),
        "<h1>Documentation</h1><p>The documentation site for every language.</p>\
         <a href=\"/vex/\">vex</a> <a href=\"/zim/\">zim</a>"
            .to_string(),
    );
    // Wrap: CLEAN is minimal honest HTML; MESSY adds the recurring chrome + furniture.
    inner
        .into_iter()
        .map(|(path, body)| {
            let html = if messy {
                format!("<html><body>{CHROME_TOP}{body}{FURNITURE}{CHROME_BOTTOM}</body></html>")
            } else {
                format!("<html><body>{body}</body></html>")
            };
            (path, html)
        })
        .collect()
}

/// Serve a generated site from localhost; returns the base url (`http://127.0.0.1:port`).
/// Exact-path routing up to a trailing slash (the crawler canonicalizes `/vex/` to `/vex`, as
/// real servers treat both as one resource), 404 for anything the site does not define — the
/// crawler must find pages through the index links, never by accident.
fn serve_site(site: BTreeMap<String, String>) -> String {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind localhost");
    let port = listener.local_addr().expect("addr").port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]).into_owned();
            let path = req
                .split_whitespace()
                .nth(1)
                .unwrap_or("/")
                .split(['?', '#'])
                .next()
                .unwrap_or("/");
            let canonical = if path.len() > 1 { path.trim_end_matches('/') } else { path };
            let page = site.get(canonical).or_else(|| site.get(&format!("{canonical}/")));
            match page {
                Some(body) => {
                    let _ = write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                }
                None => {
                    let _ = write!(
                        stream,
                        "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    );
                }
            }
        }
    });
    format!("http://127.0.0.1:{port}")
}

// ── Classification — the two axes, observed out-of-process ────────────────────────────────────

/// A fact's OBSERVED enforcement state. `Proven`/`Graded` both mean "fires on the planted bad
/// file with the required citation, clean file silent" — the tier is read off the fact's web
/// node state. `Quiet` means the conservation ledger names why no detector fires. `Silent` is
/// the failure class the law forbids: neither fired nor named.
#[derive(Clone, Debug)]
enum Observed {
    Proven,
    Graded,
    Quiet { id: String, reason: String },
    Silent,
}

/// One census run over a trained render: the offline verdict, each language's conservation
/// ledger, and each counted fact's knowledge-probe answer (`lint_query kind=web`).
struct Census {
    verdict: String,
    /// `lang → [(id, reason)]` from `lint_query kind=rules` → `withheld[]`.
    ledgers: BTreeMap<&'static str, Vec<(String, String)>>,
    /// `fact id → lint_query kind=web` JSON for the fact's subject, scoped to its language.
    webs: BTreeMap<&'static str, serde_json::Value>,
}

impl Census {
    /// KNOWLEDGE AXIS probe — true when the machine KNOWS the fact: its subject has a node in
    /// its language's web whose verbatim governing prose names the subject and carries the
    /// docs' negative polarity ("never"). Observed via the EXISTING out-of-process surface
    /// `lint_query kind=web arg=<subject> language=<lang>` — adequate as-is (found + verbatim
    /// `governing_prose` + PROVEN/GRADED/READ state), so no new query kind is defined.
    ///
    /// TODO (src contract — knowledge axis, owner ruling 2026-07-18): the TRAIN must put EVERY
    /// read fact into its language's web — a `ConstructNode` in at least the READ state whose
    /// `governing` carries the page's own forbidding sentence — INCLUDING subjects whose
    /// enforcement withholds: the blockless section (`flare`), the abstain-trap (`packvex`),
    /// and the member-vetoed bare shape (`grip`). A ledger row is never a reason to not-know.
    /// Implementing exactly that makes this probe answer `found: true` with governing prose
    /// for all N facts, and knowledge reaches N/N without touching this test.
    fn known(&self, f: &Fact) -> bool {
        let Some(web) = self.webs.get(f.id) else { return false };
        if web["found"] != serde_json::Value::Bool(true) {
            return false;
        }
        let prose = web["governing_prose"]
            .as_array()
            .map(|rows| {
                rows.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" ").to_lowercase()
            })
            .unwrap_or_default();
        prose.contains(f.subject) && prose.contains("never")
    }

    /// The FIRING tier of a fact's web node: PROVEN when the node says so, else graded (the
    /// evidence-graded LOW tier — also the honest bucket when the node is missing, which the
    /// knowledge gate reports separately).
    fn proven_tier(&self, f: &Fact) -> bool {
        self.webs
            .get(f.id)
            .and_then(|w| w["state"].as_str())
            .is_some_and(|s| s.starts_with("PROVEN"))
    }

    /// ENFORCEMENT AXIS classification, strictly in order: FIRES (bad file flags the subject
    /// with the required citation, clean file silent) at its web-node tier; else QUIET with the
    /// ledger's NAMED reason (id contains the subject, reason non-empty); else SILENT — the
    /// forbidden class. The citation requirement is waived for the member shape: a dotted-form
    /// rule is pinned by the PARENT page's typography, and the citation law is carried by the
    /// proven facts and the blockless `flare` assertion.
    fn classify(&self, f: &Fact) -> Observed {
        let bad = format!("bad.{}", f.lang);
        let clean = format!("clean.{}", f.lang);
        let words_cited = f.kind == "member-shape"
            || forbid_words(f).map_or(true, |w| self.verdict.contains(&w));
        if !f.planted_bad().is_empty()
            && flagged_in(&self.verdict, &bad, f.subject)
            && !flagged_in(&self.verdict, &clean, f.subject)
            && words_cited
        {
            return if self.proven_tier(f) { Observed::Proven } else { Observed::Graded };
        }
        if let Some((id, reason)) = self.ledgers.get(f.lang).and_then(|rows| {
            rows.iter().find(|(id, reason)| id.contains(f.subject) && !reason.is_empty())
        }) {
            return Observed::Quiet { id: id.clone(), reason: reason.clone() };
        }
        Observed::Silent
    }

    /// True when the fact's subject is flagged NOWHERE in the verdict (the tutorial-negative
    /// and abstain-trap silence law, over both planted files).
    fn silent_everywhere(&self, f: &Fact) -> bool {
        !flagged_in(&self.verdict, &format!("bad.{}", f.lang), f.subject)
            && !flagged_in(&self.verdict, &format!("clean.{}", f.lang), f.subject)
    }
}

/// Generate one render of the site, train the REAL binary against it in a fresh hermetic
/// project, lint offline, read each language's conservation ledger, and run the knowledge probe
/// for every counted fact. Every render gets its own `TestProject` (isolated caches) and its
/// own localhost server.
fn run_census(name: &str, messy: bool, facts: &[Fact]) -> Census {
    let base = serve_site(build_site(facts, messy));
    let p = TestProject::new(name);
    // Plant one bad + one clean file per language from the manifest's planted forms.
    for lang in ["vex", "zim"] {
        let planted = |pick: fn(&Fact) -> &'static str| -> String {
            facts
                .iter()
                .filter(|f| f.lang == lang && !pick(f).is_empty())
                .map(pick)
                .collect::<Vec<_>>()
                .join("\n")
                + "\n"
        };
        p.write(&format!("bad.{lang}"), &planted(Fact::planted_bad));
        p.write(&format!("clean.{lang}"), &planted(Fact::planted_good));
    }
    // Register both languages as crawl sources — pure data, exactly the user's setup path.
    // Both seeds sit at the SITE ROOT: `doc_crawler::in_scope` prefix-scopes the crawl to the
    // seed tree, and the shared mixed-language page lives outside both `/vex/` and `/zim/` —
    // root seeding puts the whole site in scope for both tools, and page attribution by URL
    // language segment (plus per-section language binding on the mixed page) does the rest.
    p.write(
        "lint-index/sources.json",
        &format!(
            r#"{{"version": 3, "sources": [
                {{"tool": "vexdocs", "language": "vex", "kind": "crawl", "seed": "{base}/"}},
                {{"tool": "zimdocs", "language": "zim", "kind": "crawl", "seed": "{base}/"}}
            ]}}"#
        ),
    );
    // Train online against localhost (the one network step)…
    p.call(
        "lint_config",
        &format!(r#"{{"root":{:?},"action":"train"}}"#, p.root.to_string_lossy()),
        false,
    );
    // …then everything measured is the offline live path.
    let verdict = p.lint(true);
    let mut ledgers = BTreeMap::new();
    for lang in ["vex", "zim"] {
        let text = p.call(
            "lint_query",
            &format!(r#"{{"kind":"rules","arg":"{lang}"}}"#),
            true,
        );
        let json: serde_json::Value = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("lint_query kind=rules returns JSON ({e}): {text}"));
        let entries: Vec<(String, String)> = json["withheld"]
            .as_array()
            .map(|rows| {
                rows.iter()
                    .map(|r| {
                        (
                            r["id"].as_str().unwrap_or_default().to_string(),
                            r["reason"].as_str().unwrap_or_default().to_string(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        ledgers.insert(lang, entries);
    }
    // Knowledge probe per counted fact: the fact's subject in its language's web.
    let mut webs = BTreeMap::new();
    for f in facts.iter().filter(|f| f.expected != Expect::Nothing) {
        let text = p.call(
            "lint_query",
            &format!(r#"{{"kind":"web","arg":"{}","language":"{}"}}"#, f.subject, f.lang),
            true,
        );
        let json: serde_json::Value = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("lint_query kind=web returns JSON ({e}): {text}"));
        webs.insert(f.id, json);
    }
    Census { verdict, ledgers, webs }
}

/// Classify every counted fact along BOTH axes, PRINT the two accounting lines
/// (`knowledge K/N`, `enforcement: proven X + graded Y + quiet-with-reason Z + silent S`), and
/// assert the two-axis law: knowledge K == N (the hard 100% gate — the machine learns every
/// fact) and silent == 0 (every known fact fires or stands in the ledger with a named reason).
/// Facts expected `Nothing` are asserted silent and counted OUTSIDE both axes; per-kind FP laws
/// (abstain-trap fires nowhere; a bare member token never flags in clean code) hold in EVERY
/// render. Returns each counted fact paired with its observed enforcement for the caller's
/// per-render expectation asserts.
fn account(render: &str, census: &Census, facts: &[Fact]) -> Vec<(Fact, Observed)> {
    // Negatives first: a sample program states no fact — nothing may mint, nothing may fire.
    for f in facts.iter().filter(|f| f.expected == Expect::Nothing) {
        assert!(
            census.silent_everywhere(f),
            "[{render}] {}: a {} must flag NOTHING, in either file:\n{}",
            f.id,
            f.kind,
            census.verdict
        );
    }
    let counted: Vec<&Fact> = facts.iter().filter(|f| f.expected != Expect::Nothing).collect();
    // KNOWLEDGE AXIS — the hard 100% gate.
    let unknown: Vec<&&Fact> = counted.iter().filter(|f| !census.known(f)).collect();
    println!("[census {render}] knowledge {}/{}", counted.len() - unknown.len(), counted.len());
    for f in &unknown {
        println!(
            "  NOT KNOWN: {} ({} {}) subject `{}` page {} -> {}",
            f.id, f.lang, f.kind, f.subject, f.page, census.webs.get(f.id).map(|w| w.to_string()).unwrap_or_default()
        );
    }
    assert!(
        unknown.is_empty(),
        "[{render}] KNOWLEDGE AXIS (owner ruling 2026-07-18): the machine LEARNS every fact — \
         these subjects are missing from their language web (see the src TODO contract on \
         Census::known): {:?}",
        unknown.iter().map(|f| f.id).collect::<Vec<_>>(),
    );
    // ENFORCEMENT AXIS — the honest tier partition; silent is the forbidden class.
    let states: Vec<(Fact, Observed)> = counted.iter().map(|f| (**f, census.classify(f))).collect();
    let count = |pick: fn(&Observed) -> bool| states.iter().filter(|(_, s)| pick(s)).count();
    let silent: Vec<&Fact> = states
        .iter()
        .filter(|(_, s)| matches!(s, Observed::Silent))
        .map(|(f, _)| f)
        .collect();
    println!(
        "[census {render}] enforcement: proven {} + graded {} + quiet-with-reason {} + silent {}",
        count(|s| matches!(s, Observed::Proven)),
        count(|s| matches!(s, Observed::Graded)),
        count(|s| matches!(s, Observed::Quiet { .. })),
        silent.len(),
    );
    for (f, state) in &states {
        if let Observed::Quiet { id, reason } = state {
            println!("  QUIET: {} -> ledger `{id}`: {reason}", f.id);
        }
    }
    for f in &silent {
        println!("  SILENT: {} ({} {}) subject `{}` page {}", f.id, f.lang, f.kind, f.subject, f.page);
    }
    assert!(
        silent.is_empty(),
        "[{render}] ENFORCEMENT AXIS: every known fact fires at some tier or stands in the \
         ledger with a named reason — these are silent: {:?}\nledgers: {:?}\nverdict:\n{}",
        silent.iter().map(|f| f.id).collect::<Vec<_>>(),
        census.ledgers,
        census.verdict
    );
    // Owner ruling (2026-07-18, second): perfect docs leave nothing quiet — on this
    // perfectly-authored manifest EVERY stated fact fires, so named-quiet is asserted to zero
    // in BOTH renders (named-quiet stays legal only where docs under-specify, which none do).
    let quiet: Vec<&Fact> = states
        .iter()
        .filter(|(_, s)| matches!(s, Observed::Quiet { .. }))
        .map(|(f, _)| f)
        .collect();
    assert!(
        quiet.is_empty(),
        "[{render}] owner ruling (2026-07-18, second): perfect docs leave nothing quiet — \
         these facts must FIRE by their demonstrated shape: {:?}\nledgers: {:?}\nverdict:\n{}",
        quiet.iter().map(|f| f.id).collect::<Vec<_>>(),
        census.ledgers,
        census.verdict
    );
    // The graded tier may only ever be the blockless `flare` (its FiresAnyTier ratchet);
    // every other fact fires PROVEN — the census's target accounting is proven 9 + graded 0
    // (or proven 8 + graded 1 when flare grades), quiet 0, silent 0.
    let graded: Vec<&Fact> = states
        .iter()
        .filter(|(_, s)| matches!(s, Observed::Graded))
        .map(|(f, _)| f)
        .collect();
    assert!(
        graded.iter().all(|f| f.expected == Expect::FiresAnyTier),
        "[{render}] only the any-tier blockless fact may grade — these graded unexpectedly: \
         {:?}\nverdict:\n{}",
        graded.iter().map(|f| f.id).collect::<Vec<_>>(),
        census.verdict
    );
    // Per-kind FP laws, every render: the abstain trap's clean file (novel clean twin + the
    // docs' own good form) never flags; a bare member token in clean code never flags (the
    // parent's typography is dotted — only the dotted shape may fire).
    for (f, _) in &states {
        match f.kind {
            "abstain-trap" => assert!(
                !flagged_in(&census.verdict, &format!("clean.{}", f.lang), f.subject),
                "[{render}] {}: the clean twin and plain `{}` uses must NEVER flag:\n{}",
                f.id,
                f.subject,
                census.verdict
            ),
            "member-shape" => assert!(
                !flagged_in(&census.verdict, &format!("clean.{}", f.lang), f.subject),
                "[{render}] {}: a bare `{}` token in clean code must NEVER flag:\n{}",
                f.id,
                f.subject,
                census.verdict
            ),
            _ => {}
        }
    }
    states
}

/// True when the observed enforcement MATCHES the manifest's expectation (`Nothing` never
/// reaches here — negatives are asserted in [`account`]).
fn matches_expected(f: &Fact, state: &Observed) -> bool {
    match (f.expected, state) {
        (Expect::FiresProven, Observed::Proven) => true,
        (Expect::FiresAnyTier, Observed::Proven | Observed::Graded) => true,
        // The demonstrated-shape and dotted-typography facts FIRE (the fires classification
        // already requires the clean file — novel clean twin, plain uses, bare tokens — to
        // stay unflagged); the tier is read off the web node, and [`account`] separately pins
        // the graded tier to the any-tier blockless fact alone.
        (Expect::FiresShape, Observed::Proven | Observed::Graded) => true,
        (Expect::FiresDotted, Observed::Proven | Observed::Graded) => true,
        _ => false,
    }
}

// ── The two census runs ───────────────────────────────────────────────────────────────────────

/// CLEAN render — the English-substrate ratchet: minimal honest HTML must yield knowledge N/N
/// AND land every fact EXACTLY on its expected enforcement: the six reference prohibitions fire
/// PROVEN with the docs' own words; blockless `flare` fires at some tier citing its forbidding
/// sentence; abstain-trap `packvex` fires by its demonstrated shape on the planted NOVEL
/// instance (clean twin silent); member `grip` fires by the author's dotted typography (bare
/// token silent). Nothing is quiet ([`account`] asserts it).
#[test]
fn census_clean_render_100_percent() {
    let facts = manifest();
    let census = run_census("census-clean", false, &facts);
    let states = account("clean", &census, &facts);
    for (f, state) in &states {
        assert!(
            matches_expected(f, state),
            "[clean] {}: expected {:?}, observed {state:?}\nledgers: {:?}\nverdict:\n{}",
            f.id,
            f.expected,
            census.ledgers,
            census.verdict
        );
        // The any-tier fact's finding must cite the docs' own forbidding sentence.
        if f.expected == Expect::FiresAnyTier {
            let words = forbid_words(f).expect("the blockless sentence carries its clause");
            assert!(
                census.verdict.contains(&words),
                "[clean] {}: the finding must cite the docs' forbidding sentence (`{words}`):\n{}",
                f.id,
                census.verdict
            );
        }
    }
}

/// MESSY render — the accounting instrument: recurring chrome, furniture sections, member
/// pages, and the mixed-language page may cost NOTHING the law names (owner rulings
/// 2026-07-18): knowledge stays N/N, nothing is silent, nothing is quiet, only the blockless
/// fact may grade ([`account`] asserts all four), negatives stay silent, the FP laws hold, and
/// the chrome never surfaces in findings. Any residual divergence from the clean render's
/// exact expectations is printed as the punch list for dissolving the interim HTML heuristics.
#[test]
fn census_messy_render_accounting() {
    let facts = manifest();
    let census = run_census("census-messy", true, &facts);
    let states = account("messy", &census, &facts);
    // The punch list: name every fact that fell short of its clean-render expectation.
    for (f, state) in &states {
        if !matches_expected(f, state) {
            println!(
                "  [messy punch list] {}: expected {:?}, observed {state:?}",
                f.id, f.expected
            );
        }
    }
    // Chrome hygiene: nothing minted from menus/footers/css-ish text ever surfaces.
    assert!(
        !census.verdict.contains("menu_click")
            && !census.verdict.contains("footer--")
            && !census.verdict.contains("align-items"),
        "site chrome must never surface in findings:\n{}",
        census.verdict
    );
}
