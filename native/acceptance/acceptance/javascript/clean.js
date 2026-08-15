// Modern, alive JavaScript — expected findings: ZERO.
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
