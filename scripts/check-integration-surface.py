#!/usr/bin/env python3
"""NFR-008 integration guard (TASK-029, API §18.6 and §20).

1. Facade surface: `dot_logger::logger` (all features) may expose at most 60
   named items (types, traits, free functions, ...; variants, fields and
   methods are not counted). The whole surface - items, variants, fields,
   public methods with signatures, trait impls - must equal the reviewed
   snapshot; `--bless` rewrites it.
2. Recipes in `examples/recipes/`: rustfmt-clean, no support module, `unsafe`,
   user lifetime parameter, `thread::scope` or memory split, and within the
   approved line and concept limits. Each recipe is run.
3. The README quickstart block equals the L1 recipe.

Rustdoc JSON comes from the pinned Miri nightly (format 61); no other tool.
"""

import argparse
import difflib
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
NIGHTLY = "nightly-2026-10-26"
FORMAT_VERSION = 61
MAX_ITEMS = 60
SNAPSHOT = ROOT / "tests" / "api-surface" / "logger.txt"
RECIPES = ROOT / "examples" / "recipes"
NAMED = {"struct", "enum", "union", "trait", "trait_alias", "type_alias", "constant",
         "static", "function", "macro", "proc_macro"}

# name -> (cargo example, features, max lines, max concepts, L1 base for "+n").
LIMITS = {
    "l1_file.rs": ("recipe_l1_file", [], 20, 6, None),
    "l2_rt_writers.rs": ("recipe_l2_rt_writers", [], 40, 10, None),
    "l3_tracing.rs": ("recipe_l3_tracing", ["tracing-adapter"], None, None, "l1_file.rs"),
    "l3_remote.rs": ("recipe_l3_remote", ["remote"], None, None, "l1_file.rs"),
}
L3_EXTRA_LINES = 6


def rustdoc_json():
    meta = json.loads(subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps", "--locked"],
        cwd=ROOT, check=True, capture_output=True, text=True).stdout)
    target = Path(meta["target_directory"]) / "api-surface"
    env = dict(os.environ, RUSTDOCFLAGS="-Z unstable-options --output-format json")
    subprocess.run(["cargo", f"+{NIGHTLY}", "doc", "-p", "dot-logger", "--all-features",
                    "--no-deps", "--locked", "--target-dir", str(target)],
                   cwd=ROOT, env=env, check=True)
    doc = json.loads((target / "doc" / "dot_logger.json").read_text())
    if doc["format_version"] != FORMAT_VERSION:
        sys.exit(f"rustdoc JSON format {doc['format_version']}, expected {FORMAT_VERSION}")
    return doc


class Surface:
    def __init__(self, doc):
        self.idx = doc["index"]
        self.paths = doc["paths"]
        self.doc = doc

    def item(self, i):
        return self.idx[str(i)]

    @staticmethod
    def kind(item):
        return next(iter(item["inner"]))

    # -- type rendering (enough to make a signature change visible) --
    def ty(self, t):
        if t is None:
            return "()"
        if isinstance(t, str):
            return t  # "infer"
        (k, v), = t.items()
        if k == "resolved_path":
            return self.path(v)
        if k in ("generic", "primitive"):
            return v
        if k == "borrowed_ref":
            life = f"{v['lifetime']} " if v.get("lifetime") else ""
            return f"&{life}{'mut ' if v['is_mutable'] else ''}{self.ty(v['type'])}"
        if k == "raw_pointer":
            return f"*{'mut' if v['is_mutable'] else 'const'} {self.ty(v['type'])}"
        if k == "slice":
            return f"[{self.ty(v)}]"
        if k == "array":
            return f"[{self.ty(v['type'])}; {v['len']}]"
        if k == "tuple":
            return "(" + ", ".join(self.ty(x) for x in v) + ")"
        if k == "impl_trait":
            return "impl " + " + ".join(self.bound(b) for b in v)
        if k == "dyn_trait":
            parts = [self.path(x["trait"]) for x in v["traits"]]
            if v.get("lifetime"):
                parts.append(v["lifetime"])
            return "dyn " + " + ".join(parts)
        if k == "qualified_path":
            trait = f" as {self.path(v['trait'])}" if v.get("trait") else ""
            return f"<{self.ty(v['self_type'])}{trait}>::{v['name']}"
        return json.dumps(t, sort_keys=True)

    def path(self, p):
        name = p.get("path") or p.get("name")
        args = p.get("args")
        if not args:
            return name
        if "angle_bracketed" in args:
            out = []
            for a in args["angle_bracketed"]["args"]:
                (ak, av), = a.items()
                out.append(self.ty(av) if ak == "type" else
                           av if ak == "lifetime" else json.dumps(av, sort_keys=True))
            for c in args["angle_bracketed"].get("constraints", []):
                out.append(json.dumps(c, sort_keys=True))
            return f"{name}<{', '.join(out)}>" if out else name
        if "parenthesized" in args:
            pa = args["parenthesized"]
            out = f" -> {self.ty(pa['output'])}" if pa.get("output") else ""
            return f"{name}({', '.join(self.ty(x) for x in pa['inputs'])}){out}"
        return name

    def bound(self, b):
        (k, v), = b.items()
        if k == "trait_bound":
            mod = "?" if v.get("modifier") == "maybe" else ""
            return mod + self.path(v["trait"])
        if k == "outlives":
            return v
        return json.dumps(b, sort_keys=True)

    def generics(self, g):
        out = []
        for p in g["params"]:
            (k, v), = p["kind"].items()
            if k == "type" and v.get("is_synthetic"):
                continue
            if k == "type" and v["bounds"]:
                out.append(f"{p['name']}: " + " + ".join(self.bound(b) for b in v["bounds"]))
            elif k == "const":
                out.append(f"const {p['name']}: {self.ty(v['type'])}")
            else:
                out.append(p["name"])
        where = [json.dumps(w, sort_keys=True) for w in g.get("where_predicates", [])]
        text = f"<{', '.join(out)}>" if out else ""
        return text + (" where " + ", ".join(where) if where else "")

    def fn_sig(self, name, f):
        h = f["header"]
        pre = "".join(w for w, on in (("const ", h["is_const"]), ("async ", h["is_async"]),
                                      ("unsafe ", h["is_unsafe"])) if on)
        args = []
        for arg, t in f["sig"]["inputs"]:
            args.append("self" if arg == "self" and t == {"generic": "Self"} else
                        self.ty(t) if arg == "self" else f"{arg}: {self.ty(t)}")
        out = f" -> {self.ty(f['sig']['output'])}" if f["sig"]["output"] is not None else ""
        return f"{pre}fn {name}{self.generics(f['generics'])}({', '.join(args)}){out}"

    # -- public module tree under `logger` --
    def logger_items(self):
        root = self.item(self.doc["root"])
        [mid] = [i for i in root["inner"]["module"]["items"]
                 if self.item(i)["name"] == "logger"]
        out = []
        self.walk(mid, "logger", out)
        return out

    def walk(self, mid, prefix, out):
        for i in self.item(mid)["inner"]["module"]["items"]:
            it = self.item(i)
            if it["visibility"] != "public":
                continue
            k = self.kind(it)
            if k == "use":
                u = it["inner"]["use"]
                target = self.idx.get(str(u.get("id")))
                if target is None:
                    out.append((f"{prefix}::{u['name']}", "reexport", None))
                elif self.kind(target) == "module":
                    self.walk(u["id"], prefix if u.get("is_glob") else f"{prefix}::{u['name']}",
                              out)
                else:
                    out.append((f"{prefix}::{u['name']}", self.kind(target), target))
            elif k == "module":
                self.walk(i, f"{prefix}::{it['name']}", out)
            else:
                out.append((f"{prefix}::{it['name']}", k, it))

    def lines(self, items):
        out = []
        for path, kind, it in items:
            if it is None:
                out.append(f"{kind} {path}")
                continue
            inner = it["inner"][kind]
            if kind == "function":
                out.append(f"function {path}: {self.fn_sig(it['name'], inner)}")
                continue
            if kind in ("struct", "enum", "union", "trait"):
                out.append(f"{kind} {path}{self.generics(inner['generics'])}")
            else:
                out.append(f"{kind} {path}")
            if kind == "struct":
                sk = inner["kind"]
                fields = (sk.get("plain", {}).get("fields", []) if isinstance(sk, dict) else [])
                if isinstance(sk, dict) and "tuple" in sk:
                    fields = [f for f in sk["tuple"] if f is not None]
                for f in fields:
                    fi = self.item(f)
                    if fi["visibility"] == "public":
                        out.append(f"  field {fi['name']}: {self.ty(fi['inner']['struct_field'])}")
            if kind == "enum":
                for v in inner["variants"]:
                    vi = self.item(v)
                    vk = vi["inner"]["variant"]["kind"]
                    shape = ""
                    if isinstance(vk, dict) and "tuple" in vk:
                        shape = "(" + ", ".join(
                            self.ty(self.item(f)["inner"]["struct_field"])
                            for f in vk["tuple"] if f is not None) + ")"
                    elif isinstance(vk, dict) and "struct" in vk:
                        shape = " { " + ", ".join(
                            f"{self.item(f)['name']}: "
                            f"{self.ty(self.item(f)['inner']['struct_field'])}"
                            for f in vk["struct"]["fields"]) + " }"
                    out.append(f"  variant {vi['name']}{shape}")
            if kind == "trait":
                for m in inner["items"]:
                    mi = self.item(m)
                    mk = self.kind(mi)
                    if mk == "function":
                        out.append(f"  {self.fn_sig(mi['name'], mi['inner']['function'])}")
                    else:
                        out.append(f"  {mk} {mi['name']}")
            for impl in inner.get("impls", []):
                im = self.item(impl)["inner"]["impl"]
                if im.get("is_synthetic") or im.get("blanket_impl") is not None:
                    continue
                if im.get("trait") is None:
                    for m in im["items"]:
                        mi = self.item(m)
                        if mi["visibility"] == "public" and self.kind(mi) == "function":
                            out.append(f"  {self.fn_sig(mi['name'], mi['inner']['function'])}")
                        elif mi["visibility"] == "public":
                            out.append(f"  {self.kind(mi)} {mi['name']}")
                else:
                    neg = "!" if im.get("is_negative") else ""
                    out.append(f"  impl {neg}{self.path(im['trait'])}")
        return out

    def public_kinds(self):
        """Every public path of the crate (re-exports included) -> item kind."""
        if not hasattr(self, "_kinds"):
            kinds = {}
            stack = [(self.doc["root"], ("dot_logger",))]
            seen = set()
            while stack:
                mid, prefix = stack.pop()
                if (mid, prefix) in seen:
                    continue
                seen.add((mid, prefix))
                kinds[prefix] = "module"
                for i in self.item(mid)["inner"]["module"]["items"]:
                    it = self.item(i)
                    if it["visibility"] != "public":
                        continue
                    k = self.kind(it)
                    if k == "use":
                        u = it["inner"]["use"]
                        target = self.idx.get(str(u.get("id")))
                        if target is None:
                            continue
                        if self.kind(target) == "module":
                            stack.append((u["id"], prefix if u.get("is_glob")
                                          else prefix + (u["name"],)))
                        else:
                            kinds[prefix + (u["name"],)] = self.kind(target)
                    elif k == "module":
                        stack.append((i, prefix + (it["name"],)))
                    else:
                        kinds[prefix + (it["name"],)] = k
            self._kinds = kinds
        return self._kinds


def check_surface(surface, bless, snapshot):
    items = surface.logger_items()
    named = [(p, k) for p, k, _ in items if k in NAMED or k == "reexport"]
    lines = surface.lines(items)
    text = "\n".join(lines) + "\n"
    ok = True
    print(f"facade named items: {len(named)} (limit {MAX_ITEMS})")
    if len(named) > MAX_ITEMS:
        print(f"FAIL: more than {MAX_ITEMS} named items in dot_logger::logger")
        ok = False
    if bless:
        snapshot.parent.mkdir(parents=True, exist_ok=True)
        snapshot.write_text(text)
        print(f"blessed {snapshot} ({len(lines)} lines)")
    elif not snapshot.exists() or snapshot.read_text() != text:
        old = snapshot.read_text().splitlines() if snapshot.exists() else []
        print(f"FAIL: facade surface differs from {snapshot} "
              f"(review, then rerun with --bless)")
        for x in difflib.unified_diff(old, lines, "snapshot", "current", n=2, lineterm=""):
            print(f"  {x}")
        ok = False
    return ok


def code_lines(source):
    """Integration lines per §18.6: no `use`, comments, blank lines, user
    message types or their `Record` impl."""
    counted, depth, skip, in_use, block_comment = [], 0, False, False, False
    for raw in source.splitlines():
        line = raw.strip()
        if block_comment:
            block_comment = "*/" not in line
            continue
        if line.startswith("/*"):
            block_comment = "*/" not in line
            continue
        if not line or line.startswith("//"):
            continue
        if depth == 0 and not skip:
            if in_use or re.match(r"(pub\s+)?use\s", line):
                in_use = not line.endswith(";")
                continue
            if re.match(r"#\[derive|(pub\s+)?(struct|enum)\s|impl(<[^>]*>)?\s+"
                        r"([\w:]*::)?Record\s+for\s", line):
                skip = True
        opened = line.count("{") - line.count("}")
        if skip:
            depth += opened
            if depth == 0 and (line.endswith("}") or line.endswith(";")) \
                    and not line.startswith("#["):
                skip = False
            continue
        depth += opened
        counted.append(line)
    return counted


def concepts(source, surface):
    """`dot_logger` paths the code names that resolve to a type, trait or
    free function (modules are not concepts; method names are not counted)."""
    names = set()
    flat = re.sub(r"//[^\n]*", "", source)
    for m in re.finditer(r"use\s+dot_logger::([^;]+);", flat):
        names |= expand_use(m.group(1).replace("\n", " "))
    for m in re.finditer(r"(?<!use )\bdot_logger::((?:\w+::)*\w+)", flat):
        names.add(tuple(m.group(1).split("::")))
    found = {}
    kinds = surface.public_kinds()
    for parts in names:
        if parts[-1] in ("self", "*"):
            continue
        kind = kinds.get(("dot_logger", *parts))
        if kind is None:
            print(f"FAIL: cannot resolve dot_logger::{'::'.join(parts)}")
            found["?" + "::".join(parts)] = "unresolved"
        elif kind != "module":
            found["::".join(parts)] = kind
    return found


def expand_use(tree, prefix=()):
    tree = tree.strip()
    out = set()
    if "{" not in tree:
        parts = tuple(x.strip() for x in tree.split("::") if x.strip())
        out.add(prefix + parts)
        return out
    head, body = tree.split("{", 1)
    body = body.rsplit("}", 1)[0]
    base = prefix + tuple(x.strip() for x in head.split("::") if x.strip())
    depth, cur = 0, ""
    for ch in body + ",":
        if ch == "," and depth == 0:
            if cur.strip():
                out |= expand_use(cur, base)
            cur = ""
            continue
        depth += (ch == "{") - (ch == "}")
        cur += ch
    return out


FORBIDDEN = [
    (r"\bmod\s+support\b|support::|#\[path", "support module"),
    (r"\bunsafe\b", "unsafe"),
    (r"<\s*'[a-z_]\w*|'(?!static\b)[a-z_]\w*\b(?!')", "user lifetime parameter"),
    (r"thread::scope", "thread::scope"),
    (r"MemoryBudget|SessionConfig|\b(input|raw|output|external|control|worker)_bytes\b",
     "memory split"),
]


def check_recipes(surface, run):
    ok = True
    files = sorted(RECIPES.glob("*.rs")) if RECIPES.exists() else []
    if not files:
        print(f"FAIL: no recipes in {RECIPES.relative_to(ROOT)}")
        return False
    missing = sorted(set(LIMITS) - {f.name for f in files})
    for name in missing:
        print(f"FAIL: recipe {name} missing")
        ok = False
    counts = {}
    for f in files:
        if f.name not in LIMITS:
            print(f"FAIL: {f.name} has no limits in this script")
            ok = False
            continue
        source = f.read_text()
        fmt = subprocess.run(["rustfmt", "--check", "--edition", "2024", str(f)],
                             cwd=ROOT, capture_output=True, text=True)
        if fmt.returncode != 0:
            print(f"FAIL: {f.name} is not rustfmt-clean")
            ok = False
        stripped = re.sub(r"//[^\n]*", "", source)
        for pattern, what in FORBIDDEN:
            if re.search(pattern, stripped):
                print(f"FAIL: {f.name} uses {what}")
                ok = False
        counts[f.name] = (len(code_lines(source)), concepts(source, surface))
    for name, (lines, found) in counts.items():
        _, _, max_lines, max_concepts, base = LIMITS[name]
        verdict = []
        if max_lines is not None and lines > max_lines:
            verdict.append(f"lines {lines} > {max_lines}")
        if max_concepts is not None and len(found) > max_concepts:
            verdict.append(f"concepts {len(found)} > {max_concepts}")
        extra = ""
        if base is not None and base in counts:
            diff = lines - counts[base][0]
            extra = f", +{diff} over {base}"
            if diff > L3_EXTRA_LINES:
                verdict.append(f"+{diff} lines > +{L3_EXTRA_LINES}")
        print(f"{name}: {lines} lines, {len(found)} concepts "
              f"[{', '.join(sorted(found))}]{extra}")
        if verdict:
            print(f"FAIL: {name}: {'; '.join(verdict)}")
            ok = False
    if run and ok:
        for name in sorted(counts):
            example, features, *_ = LIMITS[name]
            command = ["cargo", "run", "-q", "-p", "dot-logger", "--locked",
                       "--example", example]
            if features:
                command += ["--features", ",".join(features)]
            print(f"run {example}", flush=True)
            with tempfile.TemporaryDirectory(prefix="dot-logger-recipe-") as tmp:
                result = subprocess.run(command, cwd=ROOT, env=dict(os.environ, TMPDIR=tmp))
            if result.returncode != 0:
                print(f"FAIL: {example} did not exit successfully")
                ok = False
    return ok


def check_readme():
    """The README quickstart is the L1 recipe verbatim."""
    text = (ROOT / "README.md").read_text()
    blocks = re.findall(r"<!-- recipe:([\w.]+) -->\n```rust\n(.*?)```\n<!-- /recipe -->",
                        text, re.S)
    if not blocks:
        print("FAIL: README.md has no recipe block")
        return False
    ok = True
    for name, body in blocks:
        if body != (RECIPES / name).read_text():
            print(f"FAIL: README.md block differs from examples/recipes/{name}")
            ok = False
    print(f"README recipe blocks: {len(blocks)}")
    return ok


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--bless", action="store_true", help="rewrite the surface snapshot")
    parser.add_argument("--snapshot", type=Path, default=SNAPSHOT)
    parser.add_argument("--no-run", action="store_true", help="skip running the recipes")
    args = parser.parse_args()
    surface = Surface(rustdoc_json())
    ok = check_surface(surface, args.bless, args.snapshot.resolve())
    ok = check_recipes(surface, not args.no_run) and ok
    ok = check_readme() and ok
    print("NFR-008 integration guard:", "Pass" if ok else "FAIL")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
