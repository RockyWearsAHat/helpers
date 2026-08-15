//! THROWAWAY MEASUREMENT HARNESS (untracked): the NOTE-SCOPE read. For every deprecation-marker
//! element on the python-library corpus, replay the item attribution (`attested_item_shapes`'s three
//! typographies) and ALSO capture the note's OWN sentence text. Then classify each attributed item:
//! - EXCEPT-SCOPE: the note names the item AFTER an exception token ("except") — the note explicitly
//!   EXCLUDES this item from the deprecation (the TLSVersion.TLSv1_3 misread).
//! - CONDITIONAL: the note's deprecation clause is conditional/arg-form ("… if …", "passing …") — the
//!   note deprecates a USAGE FORM of the item, not the item (the asyncio.shield/urlencode misread).
//! - PLAIN: the note deprecates the item itself (keep).
//! Output: every (item, class, note-head) triple + summary counts, so the cut is measurable by hand.
//! Run: `cargo run --release --features crawl --example notescope_probe`
use helpers_native::lint_codec::{self, Dec};

fn decode_crawl(bytes: &[u8]) -> Option<Vec<(String, String)>> {
    let (_s, mut d) = Dec::open(bytes, lint_codec::kind::CRAWL)?;
    let _v = d.str()?;
    let _c = d.fixed_u64()?;
    let n = d.u()? as usize;
    let mut pages = Vec::with_capacity(n.min(65_536));
    for _ in 0..n {
        let url = d.str()?;
        let body = d.str()?;
        let _ = d.boolean()?;
        let _ = d.fixed_u64()?;
        let _ = d.fixed_u64()?;
        pages.push((url, body));
    }
    Some(pages)
}

fn load(name: &str) -> Vec<(String, String)> {
    let home = std::env::var("HOME").unwrap();
    let dir = format!("{home}/.cache/helpers/lint-index/crawls");
    std::fs::read(format!("{dir}/{name}.bin")).ok().and_then(|b| decode_crawl(&b)).unwrap_or_default()
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let key = format!("{name}=");
    let i = tag.find(&key)?;
    let rest = &tag[i + key.len()..];
    let q = rest.chars().next()?;
    if q != '"' && q != '\'' {
        return None;
    }
    let rest = &rest[1..];
    let end = rest.find(q)?;
    Some(rest[..end].to_string())
}

const MARKER_ANCHOR_WINDOW: usize = 20_000;

/// The note's own text: the whitespace-collapsed prose following the marker tag, up to the first
/// subsequent id-bearing tag or `cap` chars — the sentence the author rendered inside the badge.
fn note_text(body: &str, mpos: usize, cap: usize) -> String {
    let bytes = body.as_bytes();
    let mut i = mpos;
    let mut out = String::new();
    let mut in_tag = false;
    while i < bytes.len() && out.len() < cap {
        if bytes[i] == b'<' {
            let Some(rel) = body[i..].find('>') else { break };
            let tag = &body[i..i + rel + 1];
            if i > mpos && attr(tag, "id").is_some() {
                break; // next item's region begins
            }
            i += rel + 1;
            in_tag = false;
            continue;
        }
        if !in_tag {
            let c = body[i..].chars().next().unwrap();
            if c.is_whitespace() {
                if !out.ends_with(' ') && !out.is_empty() {
                    out.push(' ');
                }
            } else {
                out.push(c);
            }
            i += c.len_utf8();
        } else {
            i += 1;
        }
    }
    out
}

fn main() {
    let tokens = helpers_native::lint_attest::prohibition_class_tokens();
    assert!(!tokens.is_empty(), "no prohibition tokens — deprecation-status.json missing");
    let mut pages = load("python-library");
    pages.extend(load("python-docs"));
    println!("corpus python-library+docs: {} pages, prohibition tokens {tokens:?}", pages.len());
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let dotted_item = |v: &str| -> bool {
        v.len() >= 3
            && v.contains('.')
            && !v.starts_with('.')
            && !v.ends_with('.')
            && !v.contains("..")
            && v.chars().all(|c| is_ident(c) || c == '.')
            && v.rsplit('.').next().map(|last| last.len() >= 2).unwrap_or(false)
    };
    let (mut n_plain, mut n_except, mut n_cond) = (0usize, 0usize, 0usize);
    for (url, body) in &pages {
        // Replay of attested_item_shapes' scan (ids, markers, text positions).
        let mut ids: Vec<(usize, String)> = Vec::new();
        let mut markers: Vec<(usize, Option<String>)> = Vec::new();
        let mut text_positions: Vec<usize> = Vec::new();
        let bytes = body.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            if bytes[i] == b'<' {
                let Some(rel) = body[i..].find('>') else { break };
                let tag = &body[i..i + rel + 1];
                let own_id = attr(tag, "id");
                if let Some(id) = &own_id {
                    ids.push((i, id.clone()));
                }
                if let Some(class) = attr(tag, "class") {
                    if class.split_whitespace().any(|t| tokens.iter().any(|m| m.eq_ignore_ascii_case(t))) {
                        markers.push((i, own_id));
                    }
                }
                i += rel + 1;
            } else {
                if !(bytes[i] as char).is_whitespace() {
                    text_positions.push(i);
                }
                i += 1;
            }
        }
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (mpos, own_id) in &markers {
            let mpos = *mpos;
            let next = ids.iter().find(|(apos, _)| *apos > mpos);
            let prose_intervenes = |apos: usize| {
                let k = text_positions.partition_point(|&t| t <= mpos);
                text_positions.get(k).is_some_and(|&t| t < apos)
            };
            let attributed = match (own_id, next) {
                (Some(id), _) => Some(id),
                (None, Some((apos, id)))
                    if apos - mpos <= MARKER_ANCHOR_WINDOW && !prose_intervenes(*apos) =>
                {
                    Some(id)
                }
                _ => ids
                    .iter()
                    .rev()
                    .find(|(apos, _)| *apos < mpos && mpos - apos <= MARKER_ANCHOR_WINDOW)
                    .map(|(_, id)| id),
            };
            let Some(id) = attributed.filter(|id| dotted_item(id)) else { continue };
            if !seen.insert(id.clone()) {
                continue;
            }
            let note = note_text(body, mpos, 600);
            let lower = note.to_lowercase();
            // PASS 29 regression: the LIB's meaning-anchored read must reproduce the PASS-28 token
            // classification (computed inline below) on every attested item of both corpora.
            let lib_counter = helpers_native::lint_lang_layer::note_counter_attests(&lower, id);
            let item_lower = id.to_lowercase();
            let last = item_lower.rsplit('.').next().unwrap_or(&item_lower).to_string();
            let two = {
                let parts: Vec<&str> = item_lower.split('.').collect();
                if parts.len() >= 2 { parts[parts.len() - 2..].join(".") } else { item_lower.clone() }
            };
            let bounded_word = |hay: &str, word: &str| -> bool {
                let mut from = 0usize;
                while let Some(rel) = hay[from..].find(word) {
                    let s = from + rel;
                    let before_ok = s == 0
                        || !hay[..s].chars().next_back().unwrap().is_ascii_alphanumeric()
                            && hay[..s].chars().next_back().unwrap() != '_';
                    let after = s + word.len();
                    let after_ok = after >= hay.len()
                        || !hay[after..].chars().next().unwrap().is_ascii_alphanumeric()
                            && hay[after..].chars().next().unwrap() != '_';
                    if before_ok && after_ok {
                        return true;
                    }
                    from = after;
                }
                false
            };
            // EXCEPT-SCOPE: an exception token precedes a mention of the item in the note itself.
            let except_scope = lower.find("except").is_some_and(|e| {
                let after = &lower[e..];
                after.contains(&two) || after.contains(&format!(".{last}"))
            });
            // USAGE-FORM: the FIRST SENTENCE of the clause after the deprecation head carries a
            // usage-form token as a whole word ("if" conditional, "Passing/Setting/Accepting" gerunds).
            let usage_tokens = ["if", "passing", "setting", "accepting"];
            let clause = lower.split_once(':').map(|(_, c)| c).unwrap_or(&lower);
            let first_sentence = match clause.find(". ") {
                Some(p) => &clause[..p + 1],
                None => clause,
            };
            let usage_form =
                !except_scope && usage_tokens.iter().any(|t| bounded_word(first_sentence, t));
            // MISATTRIBUTED: the note names other dotted items that own anchors on THIS page, none of
            // them related to the attributed id, and the attributed item's own name is absent.
            let page_ids: std::collections::HashSet<String> =
                ids.iter().map(|(_, v)| v.to_lowercase()).collect();
            let note_dotted: Vec<String> = lower
                .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'))
                .filter(|w| dotted_item(w))
                .map(|w| w.trim_matches('.').to_string())
                .collect();
            let related = |a: &str, b: &str| a == b || a.ends_with(&format!(".{b}")) || b.ends_with(&format!(".{a}"));
            let misattributed = !except_scope
                && !usage_form
                && !note_dotted.is_empty()
                && note_dotted.iter().any(|w| page_ids.contains(w))
                && !note_dotted.iter().any(|w| related(w, &item_lower))
                && !bounded_word(&lower, &last);
            let class = if except_scope {
                n_except += 1;
                "EXCEPT-SCOPE"
            } else if usage_form {
                n_cond += 1;
                "USAGE-FORM"
            } else if misattributed {
                n_cond += 1;
                "MISATTRIBUTED"
            } else {
                n_plain += 1;
                "PLAIN"
            };
            let page = url.rsplit('/').next().unwrap_or(url);
            let token_counter = class == "EXCEPT-SCOPE" || class == "USAGE-FORM";
            if lib_counter != token_counter {
                println!("!! DIVERGENCE [{class}] lib_counter={lib_counter} {id}  ({page})");
                println!("    note: {}", &note[..note.len().min(240)]);
            } else if class != "PLAIN" {
                println!("[{class}] {id}  ({page})");
                println!("    note: {}", &note[..note.len().min(240)]);
            }
        }
    }
    println!("\nsummary: PLAIN {n_plain}  EXCEPT-SCOPE {n_except}  CONDITIONAL {n_cond}");
}