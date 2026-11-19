#!/usr/bin/env python3
"""Unit tests for `gen-event-mirrors.py`.

Run from the repository root:

    python3 -m unittest discover -s crates/host-flutter/scripts -p test_gen_event_mirrors.py
"""

from __future__ import annotations

import importlib.util
import io
import tempfile
import unittest
from contextlib import redirect_stderr
from pathlib import Path

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location(
    "gen_event_mirrors", HERE / "gen-event-mirrors.py"
)
assert SPEC is not None and SPEC.loader is not None
GEN = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GEN)


class PayloadStructsTest(unittest.TestCase):
    def test_reads_public_fields_in_order(self):
        text = """
#[derive(Clone, Debug, PartialEq)]
pub struct SetVolume {
    pub deck: u16,
    pub volume: f32,
    pub soft_takeover: bool,
}
"""
        self.assertEqual(
            GEN.payload_structs(text),
            [("SetVolume", [("deck", "u16"), ("volume", "f32"), ("soft_takeover", "bool")])],
        )

    def test_keeps_generic_and_option_types_intact(self):
        text = """
pub struct DeckLevels {
    pub deck: Option<u16>,
    pub peak_l: f32,
}
"""
        self.assertEqual(
            GEN.payload_structs(text),
            [("DeckLevels", [("deck", "Option<u16>"), ("peak_l", "f32")])],
        )

    def test_accepts_a_field_less_payload(self):
        text = "pub struct ToggleMasterCue {}"
        self.assertEqual(GEN.payload_structs(text), [("ToggleMasterCue", [])])

    def test_ignores_private_fields(self):
        text = """
pub struct Mixed {
    pub shown: u8,
    hidden: u8,
}
"""
        self.assertEqual(GEN.payload_structs(text), [("Mixed", [("shown", "u8")])])


class EventVariantsTest(unittest.TestCase):
    def test_reads_tuple_variants_in_order(self):
        text = """
pub enum Event {
    DeckUpdated(DeckUpdated),
    /// A notice.
    Notice(Notice),
}
"""
        self.assertEqual(
            GEN.event_variants(text),
            [("DeckUpdated", "DeckUpdated"), ("Notice", "Notice")],
        )

    def test_rejects_an_empty_enum(self):
        with self.assertRaises(SystemExit):
            GEN.event_variants("pub enum Event {\n}")

    def test_rejects_a_unit_variant(self):
        with self.assertRaises(SystemExit):
            GEN.event_variants("pub enum Event {\n    DeckUpdated,\n}")

    def test_rejects_a_struct_variant(self):
        with self.assertRaises(SystemExit):
            GEN.event_variants("pub enum Event {\n    DeckUpdated { deck: u16 },\n}")

    def test_rejects_an_unsupported_declaration(self):
        with self.assertRaises(SystemExit):
            GEN.event_variants("pub enum Event {\n    #[serde(skip)]\n    DeckUpdated(DeckUpdated),\n}")

    def test_requires_a_named_payload(self):
        with self.assertRaises(SystemExit):
            GEN.event_variants("pub enum Event {\n    DeckUpdated(u16),\n}")


class GenerateTest(unittest.TestCase):
    def setUp(self):
        self.text = GEN.generate()
        self.variants = GEN.event_variants(GEN.EVENTS.read_text())

    def test_mirrors_every_event_variant(self):
        self.assertEqual(len(self.variants), 19)
        for name, payload in self.variants:
            self.assertIn(f"    {name}({payload}),", self.text)
            self.assertIn(f"#[flutter_rust_bridge::frb(mirror({payload}))]", self.text)

    def test_mirrors_every_message_struct(self):
        structs = GEN.payload_structs(GEN.MESSAGES.read_text())
        self.assertTrue(structs)
        for name, fields in structs:
            self.assertIn(f"#[flutter_rust_bridge::frb(mirror({name}))]", self.text)
            for ident, _ty in fields:
                self.assertIn(f"    pub {ident}:", self.text)

    def test_keeps_the_flutter_surface_event_only(self):
        self.assertNotIn("mirror(Topic)", self.text)
        self.assertNotIn("_Topic", self.text)
        self.assertNotIn("mirror(Command)", self.text)
        self.assertNotIn("_Command {", self.text)
        self.assertNotIn("CommandTarget", self.text)
        self.assertNotIn("EventTopic", self.text)

    def test_is_deterministic(self):
        self.assertEqual(self.text, GEN.generate())

    def test_check_reports_a_stale_mirror(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "event_mirrors.rs"
            out.write_text("// stale\n")
            original = GEN.OUT
            GEN.OUT = out
            try:
                with redirect_stderr(io.StringIO()):
                    self.assertEqual(GEN.main(["gen-event-mirrors.py", "--check"]), 1)
                out.write_text(self.text)
                self.assertEqual(GEN.main(["gen-event-mirrors.py", "--check"]), 0)
            finally:
                GEN.OUT = original

    def test_write_produces_the_checked_text(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "event_mirrors.rs"
            original = GEN.OUT
            GEN.OUT = out
            try:
                self.assertEqual(GEN.main(["gen-event-mirrors.py"]), 0)
                self.assertEqual(out.read_text(), self.text)
            finally:
                GEN.OUT = original


if __name__ == "__main__":
    unittest.main()
