#!/usr/bin/env python3
"""NFR-008 integration guard (TASK-029, API §18.6 and §20).

1. Facade surface: `dot_logger::logger` (all features) may expose at most 60
   named items (types, traits, free functions, ...; variants, fields and
   methods are not counted). The whole surface - items, variants, fields,
   public methods with signatures, trait bounds and associated items, impl
   targets and conditions - must equal the reviewed snapshot; `--bless`
   rewrites it. Item shapes the renderer does not know fail.
2. Recipes in `examples/recipes/`: rustfmt-clean, no support module, `unsafe`,
   user lifetime parameter, `thread::scope` or memory split, and within the
   approved line and concept limits (concepts resolved through `use`
   bindings; glob imports and unresolved paths fail). Each recipe is run.
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
                out.append(self.constraint(c))
            return f"{name}<{', '.join(out)}>" if out else name
        if "parenthesized" in args:
            pa = args["parenthesized"]
            out = f" -> {self.ty(pa['output'])}" if pa.get("output") else ""
            return f"{name}({', '.join(self.ty(x) for x in pa['inputs'])}){out}"
        return name

    def constraint(self, c):
        (k, v), = c["binding"].items()
        name = c["name"] + (self.path({"path": "", "args": c["args"]}) if c.get("args") else "")
        if k == "equality":
            (tk, tv), = v.items()
            return f"{name} = {self.ty(tv) if tk == 'type' else json.dumps(tv, sort_keys=True)}"
        return f"{name}: {self.bounds(v)}"

    def bound(self, b):
        (k, v), = b.items()
        if k == "trait_bound":
            mod = {"none": "", "maybe": "?", "maybe_const": "~const "}.get(
                v.get("modifier"), f"{v.get('modifier')} ")
            return self.hrtb(v.get("generic_params")) + mod + self.path(v["trait"])
        if k == "outlives":
            return v
        return json.dumps(b, sort_keys=True)

    def bounds(self, bs):
        return " + ".join(self.bound(b) for b in bs)

    def hrtb(self, params):
        return f"for<{', '.join(self.params(params))}> " if params else ""

    def params(self, params):
        out = []
        for p in params:
            (k, v), = p["kind"].items()
            if k == "type" and v.get("is_synthetic"):
                continue
            if k == "lifetime":
                out.append(p["name"] + (": " + " + ".join(v["outlives"]) if v["outlives"] else ""))
            elif k == "type":
                text = p["name"] + (f": {self.bounds(v['bounds'])}" if v["bounds"] else "")
                out.append(text + (f" = {self.ty(v['default'])}" if v.get("default") else ""))
            elif k == "const":
                default = f" = {v['default']}" if v.get("default") else ""
                out.append(f"const {p['name']}: {self.ty(v['type'])}{default}")
            else:
                out.append(json.dumps(p, sort_keys=True))
        return out

    def predicate(self, w):
        (k, v), = w.items()
        if k == "bound_predicate":
            return f"{self.hrtb(v.get('generic_params'))}{self.ty(v['type'])}: " \
                   f"{self.bounds(v['bounds'])}"
        if k == "lifetime_predicate":
            return f"{v['lifetime']}: {' + '.join(v['outlives'])}"
        if k == "eq_predicate":
            (rk, rv), = v["rhs"].items()
            rhs = self.ty(rv) if rk == "type" else json.dumps(rv, sort_keys=True)
            return f"{self.ty(v['lhs'])} = {rhs}"
        return json.dumps(w, sort_keys=True)

    def generics(self, g, split=False):
        """`<params>` and ` where ...`; joined unless `split`."""
        out = self.params(g["params"])
        where = [self.predicate(w) for w in g.get("where_predicates", [])]
        text = f"<{', '.join(out)}>" if out else ""
        clause = " where " + ", ".join(where) if where else ""
        return (text, clause) if split else text + clause

    def fn_sig(self, name, f, trait_item=False):
        h = f["header"]
        pre = "".join(w for w, on in (("const ", h["is_const"]), ("async ", h["is_async"]),
                                      ("unsafe ", h["is_unsafe"])) if on)
        args = []
        for arg, t in f["sig"]["inputs"]:
            args.append("self" if arg == "self" and t == {"generic": "Self"} else
                        self.ty(t) if arg == "self" else f"{arg}: {self.ty(t)}")
        out = f" -> {self.ty(f['sig']['output'])}" if f["sig"]["output"] is not None else ""
        params, where = self.generics(f["generics"], split=True)
        body = " { .. }" if trait_item and f.get("has_body") else ""
        return f"{pre}fn {name}{params}({', '.join(args)}){out}{where}{body}"

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
        """One line per contract element. An item shape this guard cannot
        render fails instead of being abbreviated (REV-054 RV-134)."""
        out = []
        for path, kind, it in items:
            if it is None:
                out.append(f"{kind} {path}")
                continue
            inner = it["inner"][kind]
            attrs = "#[non_exhaustive] " if "non_exhaustive" in it.get("attrs", []) else ""
            if kind == "function":
                out.append(f"function {path}: {self.fn_sig(it['name'], inner)}")
                continue
            if kind == "type_alias":
                params, where = self.generics(inner["generics"], split=True)
                out.append(f"type {path}{params}{where} = {self.ty(inner['type'])}")
                continue
            if kind == "constant":
                out.append(f"const {path}: {self.ty(inner['type'])}")
                continue
            if kind == "static":
                mut = "mut " if inner.get("is_mutable") else ""
                out.append(f"static {mut}{path}: {self.ty(inner['type'])}")
                continue
            if kind == "trait":
                pre = ("unsafe " if inner["is_unsafe"] else "") + \
                      ("auto " if inner["is_auto"] else "")
                params, where = self.generics(inner["generics"], split=True)
                supers = f": {self.bounds(inner['bounds'])}" if inner["bounds"] else ""
                out.append(f"{pre}trait {path}{params}{supers}{where}")
                for m in inner["items"]:
                    out.append("  " + self.assoc_item(self.item(m), trait_item=True))
            elif kind in ("struct", "enum", "union"):
                out.append(f"{attrs}{kind} {path}{self.generics(inner['generics'])}")
                out.extend(self.fields(kind, inner))
            else:
                sys.exit(f"FAIL: facade item {path} is a {kind}; the surface guard "
                         "cannot render it (extend Surface.lines)")
            for impl in inner.get("impls", []):
                out.extend(self.impl_lines(self.item(impl)["inner"]["impl"]))
        return out

    def fields(self, kind, inner):
        out = []
        if kind == "enum":
            for v in inner["variants"]:
                vi = self.item(v)
                vk = vi["inner"]["variant"]["kind"]
                shape = ""
                if isinstance(vk, dict) and "tuple" in vk:
                    shape = "(" + ", ".join(
                        self.ty(self.item(f)["inner"]["struct_field"]) if f is not None
                        else "_" for f in vk["tuple"]) + ")"
                elif isinstance(vk, dict) and "struct" in vk:
                    names = [f"{self.item(f)['name']}: "
                             f"{self.ty(self.item(f)['inner']['struct_field'])}"
                             for f in vk["struct"]["fields"]]
                    if vk["struct"]["has_stripped_fields"]:
                        names.append("..")
                    shape = " { " + ", ".join(names) + " }"
                vattrs = "#[non_exhaustive] " if "non_exhaustive" in vi.get("attrs", []) else ""
                out.append(f"  variant {vattrs}{vi['name']}{shape}")
            if inner["has_stripped_variants"]:
                out.append("  variant ..")
            return out
        if kind == "union":
            fields, stripped = inner["fields"], inner["has_stripped_fields"]
        else:
            sk = inner["kind"]
            if sk == "unit":
                return out
            if "tuple" in sk:
                fields = [f for f in sk["tuple"] if f is not None]
                stripped = len(fields) < len(sk["tuple"])
            else:
                fields, stripped = sk["plain"]["fields"], sk["plain"]["has_stripped_fields"]
        for f in fields:
            fi = self.item(f)
            if fi["visibility"] == "public":
                out.append(f"  field {fi['name']}: {self.ty(fi['inner']['struct_field'])}")
            else:
                stripped = True
        if stripped:
            out.append("  field .. (private)")
        return out

    def assoc_item(self, mi, trait_item=False):
        mk = self.kind(mi)
        v = mi["inner"][mk]
        if mk == "function":
            return self.fn_sig(mi["name"], v, trait_item)
        if mk == "assoc_const":
            default = " = .." if trait_item and v.get("value") is not None else ""
            return f"const {mi['name']}: {self.ty(v['type'])}{default}"
        if mk == "assoc_type":
            params, where = self.generics(v["generics"], split=True)
            bounds = f": {self.bounds(v['bounds'])}" if v["bounds"] else ""
            value = f" = {self.ty(v['type'])}" if v.get("type") is not None else ""
            return f"type {mi['name']}{params}{bounds}{where}{value}"
        sys.exit(f"FAIL: associated item {mi['name']} is a {mk}; the surface guard "
                 "cannot render it (extend Surface.assoc_item)")

    def impl_lines(self, im):
        if im.get("is_synthetic") or im.get("blanket_impl") is not None:
            return []
        params, where = self.generics(im["generics"], split=True)
        unsafe = "unsafe " if im["is_unsafe"] else ""
        target = self.ty(im["for"])
        if im.get("trait") is not None:
            neg = "!" if im.get("is_negative") else ""
            return [f"  {unsafe}impl{params} {neg}{self.path(im['trait'])} for {target}{where}"]
        out = []
        if params or where:
            out.append(f"  {unsafe}impl{params} {target}{where}")
        for m in im["items"]:
            mi = self.item(m)
            if mi["visibility"] == "public":
                out.append("  " + self.assoc_item(mi))
        return out

    def public_items(self):
        """Every public path of the crate (re-exports included) -> (kind, id)."""
        if not hasattr(self, "_kinds"):
            kinds = {}
            stack = [(self.doc["root"], ("dot_logger",))]
            seen = set()
            while stack:
                mid, prefix = stack.pop()
                if (mid, prefix) in seen:
                    continue
                seen.add((mid, prefix))
                kinds[prefix] = ("module", mid)
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
                            kinds[prefix + (u["name"],)] = (self.kind(target), u["id"])
                    elif k == "module":
                        stack.append((i, prefix + (it["name"],)))
                    else:
                        kinds[prefix + (it["name"],)] = (k, i)
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


def strip_code(source):
    """Source with comments and string contents blanked (newlines kept).
    Char literals are not parsed: recipes have no `'"'`."""
    out, i, n = [], 0, len(source)
    while i < n:
        two = source[i:i + 2]
        if two == "//":
            j = source.find("\n", i)
            i = n if j < 0 else j
        elif two == "/*":
            j = source.find("*/", i + 2)
            j = n if j < 0 else j + 2
            out.append(re.sub(r"[^\n]", " ", source[i:j]))
            i = j
        elif source[i] == '"':
            j = i + 1
            while j < n and source[j] != '"':
                j += 2 if source[j] == "\\" else 1
            out.append('""')
            i = j + 1
        else:
            out.append(source[i])
            i += 1
    return "".join(out)


USE = re.compile(r"\buse\s+([^;]+);")
EXTERN = re.compile(r"\bextern\s+crate\s+dot_logger\s+as\s+(\w+)\s*;")
PATH = re.compile(r"(?<![\w:])(?:::)?[A-Za-z_]\w*(?:\s*::\s*[A-Za-z_]\w*)+")
FOREIGN_ROOTS = {"std", "core", "alloc", "crate", "self", "super"}


def concepts(source, surface):
    """Types, traits and free functions of `dot_logger` the code names
    (modules are not concepts; methods, variants and associated items count
    as their owner). Paths are resolved through `use` bindings - module,
    alias, nested and crate-alias forms (REV-054 RV-135). Returns
    (`{path: kind}` deduplicated by item, errors); glob imports and
    unresolved `dot_logger` paths are errors."""
    flat = strip_code(source)
    items = surface.public_items()
    binds = {"dot_logger": ("dot_logger",)}
    binds.update({m.group(1): ("dot_logger",) for m in EXTERN.finditer(flat)})
    found, errors = {}, []

    def resolve(full, text, exact):
        for k in range(len(full), 0, -1):
            hit = items.get(full[:k])
            if hit is None:
                continue
            kind, ident = hit
            if kind == "module" and k < len(full) or exact and k < len(full):
                break
            if kind != "module":
                name = "::".join(full[1:k])
                old = found.get(ident)
                found[ident] = (min(name, old[0]) if old else name, kind)
            return full[:k], kind
        errors.append(f"cannot resolve {text}")
        return None, None

    pending = [leaf for m in USE.finditer(flat) for leaf in use_leaves(m.group(1))]
    progress = True
    while progress:
        progress = False
        for leaf in list(pending):
            parts, alias, glob = leaf
            if parts[0] not in binds:
                if parts[0] in FOREIGN_ROOTS:
                    pending.remove(leaf)
                continue
            pending.remove(leaf)
            progress = True
            full = binds[parts[0]] + parts[1:]
            text = "use " + "::".join(full) + ("::*" if glob else "")
            if glob:
                errors.append(f"{text}: glob imports are not supported by the concept count")
                continue
            path, _ = resolve(full, text, exact=True)
            if path is not None and alias != "_":
                binds[alias or path[-1]] = path
    body = USE.sub(" ", flat)
    for m in PATH.finditer(body):
        parts = tuple(x for x in re.split(r"\s*::\s*", m.group(0)) if x)
        if parts[0] in binds and len(parts) > 1:
            resolve(binds[parts[0]] + parts[1:], m.group(0), exact=False)
    return {name: kind for name, kind in found.values()}, errors


def use_leaves(tree, prefix=()):
    """`a::{b, c::{self as d}, e::*}` -> [(path, alias, is_glob)]."""
    tree = " ".join(tree.split())
    if "{" not in tree:
        alias = None
        m = re.fullmatch(r"(.*?)\s+as\s+(\w+)", tree)
        if m:
            tree, alias = m.group(1), m.group(2)
        parts = prefix + tuple(x.strip() for x in tree.split("::") if x.strip())
        if parts and parts[-1] == "*":
            return [(parts[:-1], None, True)]
        if parts and parts[-1] == "self":
            parts = parts[:-1]
        return [(parts, alias, False)] if parts else []
    head, body = tree.split("{", 1)
    body = body.rsplit("}", 1)[0]
    base = prefix + tuple(x.strip() for x in head.split("::") if x.strip())
    out, depth, cur = [], 0, ""
    for ch in body + ",":
        if ch == "," and depth == 0:
            if cur.strip():
                out += use_leaves(cur, base)
            cur = ""
            continue
        depth += (ch == "{") - (ch == "}")
        cur += ch
    return out


def limit_verdict(name, lines, found, base_lines):
    """Failures of one recipe against its §18.6 limits."""
    _, _, max_lines, max_concepts, _ = LIMITS[name]
    verdict = []
    if max_lines is not None and lines > max_lines:
        verdict.append(f"lines {lines} > {max_lines}")
    if max_concepts is not None and len(found) > max_concepts:
        verdict.append(f"concepts {len(found)} > {max_concepts}")
    if base_lines is not None and lines - base_lines > L3_EXTRA_LINES:
        verdict.append(f"+{lines - base_lines} lines > +{L3_EXTRA_LINES}")
    return verdict


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
        found, errors = concepts(source, surface)
        for error in errors:
            print(f"FAIL: {f.name}: {error}")
            ok = False
        counts[f.name] = (len(code_lines(source)), found)
    for name, (lines, found) in counts.items():
        base = LIMITS[name][4]
        base_lines = counts[base][0] if base in counts else None
        verdict = limit_verdict(name, lines, found, base_lines)
        extra = f", +{lines - base_lines} over {base}" if base_lines is not None else ""
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
