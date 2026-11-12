#!/usr/bin/env python3
"""Self-tests for scripts/check-integration-surface.py (REV-054 RV-134/135).

RV-134: contract changes (trait bounds, associated item types, impl targets
and conditions, ...) must change the surface lines; body-only changes must
not. Each case compiles a minimal crate with the pinned rustdoc JSON.
RV-135: recipe concepts must be counted through module, alias and nested
imports; unsupported forms (glob) and unresolved paths must fail.

Run: python3 scripts/test_check_integration_surface.py
"""

import importlib.util
import sys
import subprocess
import tempfile
import unittest
from pathlib import Path
import json

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("guard", HERE / "check-integration-surface.py")
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)


def crate_lines(source):
    """Surface lines of `pub mod logger` in a one-file crate."""
    with tempfile.TemporaryDirectory(prefix="dot-logger-surface-test-") as tmp:
        lib = Path(tmp) / "lib.rs"
        lib.write_text(source)
        subprocess.run(["rustdoc", f"+{guard.NIGHTLY}", "-Z", "unstable-options",
                        "--output-format", "json", "--edition", "2024", "--crate-type", "lib",
                        "--crate-name", "probe", "-o", tmp, str(lib)],
                       check=True, capture_output=True, text=True)
        doc = json.loads((Path(tmp) / "probe.json").read_text())
    assert doc["format_version"] == guard.FORMAT_VERSION
    surface = guard.Surface(doc)
    return surface.lines(surface.logger_items())


BASE = {
    "trait": "pub trait Record {",
    "const": "    const SCHEMA: u64;",
    "assoc_type": "    type Out: Clone;",
    "method": "    fn encode(&self);",
    "provided": "    fn name(&self) -> u8 { 0 }",
    "struct": "pub struct Channel<T>(core::marker::PhantomData<T>);",
    "inherent": "impl<T> Channel<T> {",
    "body": "        0",
    "copy": "impl<T> Copy for Channel<T> {}",
    "clone": "impl<T> Clone for Channel<T> { fn clone(&self) -> Self { *self } }",
    "open": "pub struct Config { pub depth: u32 }",
    "alias": "pub type Id = u32;",
}

TEMPLATE = """pub mod logger {{
    {trait}
    {const}
    {assoc_type}
    {method}
    {provided}
    }}
    {struct}
    {inherent}
        pub fn id(&self) -> u32 {{
    {body}
        }}
    }}
    {copy}
    {clone}
    {open}
    {alias}
}}
"""


def render(**change):
    return TEMPLATE.format(**dict(BASE, **change))


class SurfaceContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.base = crate_lines(render())

    def assert_changes(self, **change):
        self.assertNotEqual(crate_lines(render(**change)), self.base, change)

    def test_trait_supertrait_bound(self):
        self.assert_changes(trait="pub trait Record: 'static {")

    def test_assoc_const_type(self):
        self.assert_changes(const="    const SCHEMA: u32;")

    def test_assoc_type_bound(self):
        self.assert_changes(assoc_type="    type Out: Clone + Send;")

    def test_provided_method_becomes_required(self):
        self.assert_changes(provided="    fn name(&self) -> u8;")

    def test_trait_impl_condition(self):
        self.assert_changes(copy="impl<T: 'static> Copy for Channel<T> {}")

    def test_trait_impl_where_clause(self):
        self.assert_changes(copy="impl<T> Copy for Channel<T> where T: Send {}")

    def test_trait_impl_target(self):
        self.assert_changes(copy="impl Copy for Channel<u8> {}",
                            clone="impl Clone for Channel<u8> { fn clone(&self) -> Self "
                                  "{ *self } }")

    def test_inherent_impl_condition(self):
        self.assert_changes(inherent="impl<T: Copy> Channel<T> {")

    def test_struct_gains_private_field(self):
        self.assert_changes(open="pub struct Config { pub depth: u32, _p: () }")

    def test_type_alias_target(self):
        self.assert_changes(alias="pub type Id = u64;")

    def test_body_only_change_is_invisible(self):
        self.assertEqual(crate_lines(render(body="        1 + 1")), self.base)


L1 = (guard.RECIPES / "l1_file.rs").read_text()
L1_IMPORT = "use dot_logger::logger::{EncodeError, Logger, Preset, Record, Retention, Schema};"
L1_MATCH = "shutdown if shutdown.is_success() => Ok(()),"


class RecipeConcepts(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.surface = guard.Surface(guard.rustdoc_json())

    def concepts(self, source):
        return guard.concepts(source, self.surface)

    def test_original_l1(self):
        found, errors = self.concepts(L1)
        self.assertEqual(errors, [])
        self.assertEqual(sorted(found), ["logger::EncodeError", "logger::Logger",
                                         "logger::Preset", "logger::Record",
                                         "logger::Retention", "logger::Schema"])

    def test_module_import_hides_nothing(self):
        # REV-054 RV-135 mutation: 7 concepts, 20 lines, must exceed L1.
        source = L1.replace(L1_IMPORT, "use dot_logger::logger;\nuse logger::{EncodeError, "
                            "Logger, Preset, Record, Retention, Schema};")
        source = source.replace(L1_MATCH, "shutdown if logger::Shutdown::is_success("
                                "&shutdown) => Ok(()),")
        self.assertNotEqual(source, L1)
        found, errors = self.concepts(source)
        self.assertEqual(errors, [])
        self.assertEqual(len(found), 7)
        self.assertIn("logger::Shutdown", found)
        verdict = guard.limit_verdict("l1_file.rs", len(guard.code_lines(source)), found, None)
        self.assertTrue(any("concepts 7 > 6" in v for v in verdict), verdict)
        self.assertEqual(guard.limit_verdict("l1_file.rs", len(guard.code_lines(L1)),
                                             self.concepts(L1)[0], None), [])

    def test_alias_and_nested_imports(self):
        source = ("use dot_logger::{logger::{self as lg, Logger as L}};\n"
                  "fn f() { let _ = L::builder(); let _: lg::Preset; lg::Shutdown::x(); }")
        found, errors = self.concepts(source)
        self.assertEqual(errors, [])
        self.assertEqual(sorted(found), ["logger::Logger", "logger::Preset",
                                         "logger::Shutdown"])

    def test_crate_alias(self):
        found, errors = self.concepts("use dot_logger as dl;\nfn f() { dl::logger::Logger::x(); }")
        self.assertEqual(errors, [])
        self.assertEqual(sorted(found), ["logger::Logger"])

    def test_glob_import_fails(self):
        _, errors = self.concepts("use dot_logger::logger::*;\nfn main() {}")
        self.assertTrue(errors)
        _, errors = self.concepts("use dot_logger::logger;\nuse logger::*;\nfn main() {}")
        self.assertTrue(errors)

    def test_unresolved_paths_fail(self):
        _, errors = self.concepts("use dot_logger::logger::Nope;")
        self.assertTrue(errors)
        _, errors = self.concepts("use dot_logger::logger;\nfn f() { logger::Nope::x(); }")
        self.assertTrue(errors)

    def test_comments_and_strings_ignored(self):
        found, errors = self.concepts(
            '// use dot_logger::logger::Shutdown;\nfn f() { let _ = "dot_logger::logger::Nope"; }')
        self.assertEqual((found, errors), ({}, []))


if __name__ == "__main__":
    unittest.main()
