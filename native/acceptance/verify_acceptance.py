#!/usr/bin/env python3
"""Run the deployed linter over each language's acceptance dir and machine-diff against the
manifest: every planted rule must fire at its exact line (FN otherwise); no rule may fire in the
clean file (FP); extra fires on bad lines are reported for truth-checking."""
import json, os, re, subprocess, sys

BIN = os.environ.get("HELPERS_BIN", os.path.expanduser("~/bin/helpers-native"))
BASE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "acceptance")

def lint(root, module):
    out = subprocess.run([BIN, "call", "lint"], input=json.dumps({"root": root, "modules": [module], "max": 500}),
                         capture_output=True, text=True)
    return json.loads(json.loads(out.stdout)["content"][0]["text"] if False else json.loads(out.stdout)["content"][0]["text"])

def lint_text(root, module):
    out = subprocess.run([BIN, "call", "lint"], input=json.dumps({"root": root, "modules": [module], "max": 500}),
                         capture_output=True, text=True)
    return json.loads(out.stdout)["content"][0]["text"]

overall_ok = True
for lang in sys.argv[1:] or ["javascript", "typescript", "css", "html", "rust"]:
    d = os.path.join(BASE, lang)
    manifest = json.load(open(os.path.join(d, "manifest.json")))
    text = lint_text(d, lang)
    fired = {}
    for m in re.finditer(r"\[(?:high|medium|low)\] \[([^\]]+)\][^\n]*?(?:L(\d+)|×\d+ \(lines ([\d, ]+)\))", text):
        rid = m.group(1); lines = set()
        if m.group(2): lines.add(int(m.group(2)))
        if m.group(3): lines |= {int(x) for x in m.group(3).split(",")}
        fired.setdefault(rid, set()).update(lines)
    missed = {r: v["line"] for r, v in manifest.items() if r not in fired or v["line"] not in fired[r]}
    clean_hit = re.search(r"clean\.\w+", text) is not None
    planted_lines = {v["line"] for v in manifest.values()}
    extra = {r: sorted(ls - planted_lines) for r, ls in fired.items() if ls - planted_lines}
    ok = not missed and not clean_hit
    overall_ok &= ok
    print(f"{lang}: {'PASS' if ok else 'FAIL'} | fired {len(manifest) - len(missed)}/{len(manifest)} at exact lines"
          f" | clean file hit: {clean_hit} | extra fires: {extra if extra else '-'}")
    if missed:
        print(f"  MISSED: {missed}")
print("OVERALL:", "PASS" if overall_ok else "FAIL")
