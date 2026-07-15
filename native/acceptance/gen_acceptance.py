#!/usr/bin/env python3
"""Generate the six-language acceptance suite from the ENFORCED fire shapes (upstream-seeded):
per language a bad file planting every enforced construct (each must flag at its exact line, with
citation) and a clean modern file (must stay zero). Manifests written for exact machine diffing."""
import json, os, re, subprocess

BIN = os.environ.get("HELPERS_BIN", os.path.expanduser("~/bin/helpers-native"))
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "acceptance")

def rules_for(lang):
    out = subprocess.run([BIN, "call", "lint_query"], input=json.dumps({"kind": "rules", "arg": lang}),
                         capture_output=True, text=True)
    body = json.loads(json.loads(out.stdout)["content"][0]["text"])
    res = []
    for r in body.get("module_rules") or []:
        m = re.search(r"uses_construct\(([^)]*)\)", r.get("detector", ""))
        if m:
            res.append((r["id"], m.group(1)))
    return res

# ── per-language line templates: construct shape → one valid planted line ──
def js_line(c):
    if c == "with":
        return "with (config) { total = value; }"
    if c == "arguments.callee":
        return "function recurse(n) { return n <= 1 ? 1 : n * arguments.callee(n - 1); }"
    if c.startswith("RegExp."):
        return f"const m{abs(hash(c))%997} = {c};"
    if c.startswith("document."):
        prop = c.split(".")[1]
        calls = {"createEvent": '("Event")', "createTouch": "(window, el, 1, 0, 0)", "createTouchList": "(t)",
                 "enableStyleSheetsForSet": '("s")', "requestStorageAccessFor": '("https://a.example")',
                 "execCommand": '("bold")', "queryCommandEnabled": '("bold")', "queryCommandState": '("bold")',
                 "queryCommandSupported": '("bold")', "writeln": '("<b>x</b>")', "write": '("<b>x</b>")',
                 "browsingTopics": "()"}
        return f"{c}{calls[prop]};" if prop in calls else f"const v{abs(hash(c))%997} = {c};"
    if c.startswith(".__"):
        return f'obj{c}("prop", fn);'
    if c.startswith("."):
        m = c[1:]
        if m in ("getYear", "setYear"):
            return f"const y{abs(hash(c))%97} = when{c}({'88' if m == 'setYear' else ''});"
        if m == "compile":
            return 're.compile("pattern", "g");'
        return f'const s{abs(hash(c))%997} = text{c}({"" if m not in ("substr", "anchor", "link", "fontcolor", "fontsize") else chr(34) + "a" + chr(34)});'
    return f'{c}("a b c");'  # bare global fn: escape / unescape / createNSResolver

def css_line(c):
    values = {"-moz-float-edge": "content-box", "-moz-force-broken-image-icon": "1", "-moz-user-focus": "ignore",
              "-moz-user-input": "none", "box-align": "start", "box-direction": "reverse", "box-flex-group": "2",
              "box-lines": "multiple", "box-ordinal-group": "3", "box-orient": "horizontal", "box-pack": "center",
              "clip": "rect(0, 0, 10px, 10px)", "page-break-after": "always", "page-break-before": "avoid",
              "page-break-inside": "avoid", "text-decoration-skip": "objects", "user-modify": "read-only"}
    if c in values:
        return f".x{abs(hash(c))%997} {{ {c}: {values[c]}; }}"
    if c.startswith("::"):
        owner = {"::-moz-focus-inner": "button", "::-webkit-meter-bar": "meter"}.get(c, "div")
        return f"{owner}{c} {{ border: 0; }}"
    if c.startswith(":"):
        if c == ":host-context":
            return f"{c}(.dark) {{ color: white; }}"
        return f"img{c} {{ display: none; }}"
    return f"@media ({c}) {{ .y{abs(hash(c))%997} {{ color: red; }} }}"  # -webkit-animation/-webkit-transition

def html_line(c):
    elements = {"acronym": '<acronym title="Hyper Text">HT</acronym>', "big": "<big>large</big>",
                "center": "<center>middle</center>", "frame": '<frame src="a.html">',
                "frameset": '<frameset cols="50%,50%"></frameset>',
                "marquee": "<marquee>scrolling</marquee>", "rb": "<ruby><rb>漢</rb><rt>kan</rt></ruby>",
                "tt": "<tt>mono</tt>"}
    if c in elements:
        return elements[c]
    attrs = {"attributeType": '<animate attributeType="XML" attributeName="x"></animate>',
             "baseProfile": '<svg baseProfile="full"></svg>',
             "clip": '<svg><image clip="rect(5px, 55px, 45px, 5px)"></image></svg>',
             "requiredFeatures": '<switch><text requiredFeatures="http://www.w3.org/TR/SVG11/feature#Text">t</text></switch>',
             "version": '<svg version="1.1"></svg>', "xlink:href": '<svg><use xlink:href="#icon"></use></svg>',
             "xml:lang": '<svg><text xml:lang="en">t</text></svg>',
             "xml:space": '<svg><text xml:space="preserve"> t </text></svg>',
             "zoomAndPan": '<svg zoomAndPan="magnify"></svg>'}
    return attrs[c]

def rust_line(c):
    m = c[1:]
    exprs = {"connect": 'let joined = parts.connect(", ");', "slice_unchecked": "let piece = unsafe { s.slice_unchecked(0, 2) };",
             "trim_left": "let t1 = s.trim_left();", "trim_right": "let t2 = s.trim_right();",
             "trim_left_matches": "let t3 = s.trim_left_matches('x');", "trim_right_matches": "let t4 = s.trim_right_matches('x');",
             "abs_sub": "let d = a.abs_sub(b);", "wait_timeout_ms": "let r = cvar.wait_timeout_ms(guard, 100);",
             "compare_and_swap": "let old = cell.compare_and_swap(1, 2, Ordering::SeqCst);"}
    return exprs[m]

CLEAN = {
    "javascript": """// Modern, alive JavaScript — expected findings: ZERO.
// Prose mentions must stay silent: escape() is deprecated; avoid document.write and .substr.
const BANNER = "with arguments.callee and RegExp.input written only inside this string";
const el = document.createElement("section");
el.textContent = "hello";
document.body.appendChild(el);
const part = "hello world".substring(0, 5);
const slice2 = "hello".slice(1);
const year = new Date().getFullYear();
const re = new RegExp("p(a)t", "g");
const groups = re.exec("pat");
Object.defineProperty(el, "flag", { value: true });
const encoded = encodeURIComponent("a b c");
const evt = new CustomEvent("ready", { detail: 1 });
el.addEventListener("ready", () => {});
el.dispatchEvent(evt);
const resolver = document.createExpression ? "xpath-modern" : "none";
const cleared = new Map(); cleared.clear();
""",
    "css": """/* Modern, alive CSS — expected findings: ZERO. box-orient and clip live only in this comment. */
.layout { display: flex; align-items: flex-start; flex-direction: row-reverse; }
.layout .item { order: 3; flex: 1 1 auto; }
.card { break-after: page; break-before: avoid-page; break-inside: avoid; }
.art { clip-path: inset(0 round 8px); }
.editor { user-select: none; }
.link { text-decoration-skip-ink: auto; }
meter { appearance: none; }
button:focus-visible { outline: 2px solid blue; }
@media (prefers-reduced-motion: reduce) { .anim { transition: none; } }
""",
    "html": """<!-- Modern, alive HTML — expected findings: ZERO. <center> and xlink:href named only in this comment. -->
<main>
  <section><h1>Title</h1><p>Text with <em>emphasis</em> and <strong>weight</strong>.</p></section>
  <abbr title="Hyper Text">HT</abbr>
  <ruby>漢<rp>(</rp><rt>kan</rt><rp>)</rp></ruby>
  <iframe src="a.html" title="embedded"></iframe>
  <p style="text-align: center">centered by css</p>
  <svg viewBox="0 0 10 10" lang="en"><use href="#icon"></use><text>t</text></svg>
  <code>mono</code>
</main>
""",
    "rust": """//! Modern, alive Rust — expected findings: ZERO. trim_left and connect live only in prose.
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// Joins, trims, and compares the modern way.
pub fn modern(parts: &[&str], s: &str, cell: &AtomicUsize) -> String {
    let joined = parts.join(", ");
    let t = s.trim_start().trim_end();
    let t2 = t.trim_start_matches('x').trim_end_matches('x');
    let _ = cell.compare_exchange(1, 2, Ordering::SeqCst, Ordering::SeqCst);
    let _d = Duration::from_millis(100);
    format!("{joined}{t2}")
}
""",
}
CLEAN["typescript"] = CLEAN["javascript"].replace("const BANNER", "const BANNER: string")

HEADERS = {
    "javascript": ["// Planted: every enforced javascript construct, one per line.",
                   "const obj = {}, fn = () => {}, config = { value: 1 }, text = \"sample\", when = new Date();",
                   "const re = /x/, parts = [], el = null, t = null; let total = 0, value = 2;"],
    "css": ["/* Planted: every enforced css construct, one per line. */"],
    "html": ["<!-- Planted: every enforced html construct, one per line. -->"],
    "rust": ["//! Planted: every enforced rust construct, one per line.",
             "use std::sync::atomic::{AtomicUsize, Ordering};",
             "fn probe(parts: Vec<&str>, s: &str, a: f64, b: f64, cell: &AtomicUsize, cvar: &std::sync::Condvar, guard: std::sync::MutexGuard<bool>) {"],
}
FOOTERS = {"rust": ["}"], "javascript": [], "css": [], "html": []}
EXT = {"javascript": "js", "typescript": "ts", "css": "css", "html": "html", "rust": "rs"}
LINE = {"javascript": js_line, "typescript": js_line, "css": css_line, "html": html_line, "rust": rust_line}

for lang in ["javascript", "typescript", "css", "html", "rust"]:
    d = os.path.join(OUT, lang)
    os.makedirs(d, exist_ok=True)
    rules = rules_for(lang)
    header = HEADERS.get(lang if lang != "typescript" else "javascript", [])
    lines = list(header)
    manifest = {}
    for rid, fire in rules:
        lines.append(LINE[lang](fire))
        manifest[rid] = {"fire": fire, "line": len(lines)}
    lines += FOOTERS.get(lang if lang != "typescript" else "javascript", [])
    with open(os.path.join(d, f"bad.{EXT[lang]}"), "w") as f:
        f.write("\n".join(lines) + "\n")
    with open(os.path.join(d, f"clean.{EXT[lang]}"), "w") as f:
        f.write(CLEAN[lang if lang != "typescript" else "typescript"])
    with open(os.path.join(d, "manifest.json"), "w") as f:
        json.dump(manifest, f, indent=1)
    print(lang, len(rules), "planted")
