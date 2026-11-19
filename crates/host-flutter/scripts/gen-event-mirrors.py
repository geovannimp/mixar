#!/usr/bin/env python3
"""Generate `crates/host-flutter/src/api/event_mirrors.rs` from `bus-api`.

`flutter_rust_bridge` only scans `crates/host-flutter`, so every type defined in
`bus-api` arrives in Dart as an opaque handle. `#[frb(mirror(Original))]`
declarations in the scanned crate fix that: FRB reuses the original Rust type and
generates a real Dart class (a freezed sealed class for the `Event` enum).

Mirrors must match the original field for field. Codegen fails loudly on drift,
so a stale mirror is a build error rather than a silent divergence — but the
friendly loop is to re-run this script after editing the schema:

    python3 crates/host-flutter/scripts/gen-event-mirrors.py

`--check` compares the generated text with the file on disk without writing it,
so CI can fail on a stale mirror without a diff.

Source of truth: the payload structs in `crates/bus-api/src/messages.rs`, the
`Event` enum in `crates/bus-api/src/events.rs`, and the data types in
`crates/bus-api/src/payload.rs` / `library.rs`.

The Flutter surface is the event direction only: `Command`, `CommandTarget`,
`EventTopic`, `Arc`, the bus and its reports never cross FRB, because Dart drives
the engine through typed host methods rather than by building commands.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

CRATES = Path(__file__).resolve().parents[2]
MESSAGES = CRATES / "bus-api/src/messages.rs"
EVENTS = CRATES / "bus-api/src/events.rs"
PAYLOAD = CRATES / "bus-api/src/payload.rs"
LIBRARY = CRATES / "bus-api/src/library.rs"
OUT = CRATES / "host-flutter/src/api/event_mirrors.rs"

# Payload enums that cross FRB as mirrors, with their variants read from source.
PAYLOAD_ENUMS = [
    "EqBand",
    "FxKind",
    "JogMode",
    "PadMode",
    "SyncMode",
    "CurveSegmentKind",
]

TUPLE_VARIANT = re.compile(r"^\s*(?P<name>[A-Z]\w*)\((?P<payload>[A-Z]\w*)\),$")
ALLOWED_NON_VARIANT = ("///",)


def payload_structs(text: str) -> list[tuple[str, list[tuple[str, str]]]]:
    """Every `pub struct` in `messages.rs` with its public fields."""
    out = []
    for match in re.finditer(r"pub struct (\w+)\s*\{([^}]*)\}", text):
        name = match.group(1)
        fields = []
        for line in match.group(2).splitlines():
            line = line.strip()
            if not line.startswith("pub ") or ":" not in line:
                continue
            line = line[len("pub ") :].rstrip(",")
            ident, ty = line.split(":", 1)
            fields.append((ident.strip(), ty.strip()))
        out.append((name, fields))
    return out


def event_variants(text: str) -> list[tuple[str, str]]:
    """`[(Name, Payload), …]` from `pub enum Event`.

    Tuple variants only, no attributes we cannot mirror. Anything else in the
    body is a schema the mirror cannot represent, so it is an error rather than
    a silent skip.
    """
    match = re.search(r"pub enum Event \{(?P<body>.*?)\n\}", text, re.S)
    if not match:
        raise SystemExit(f"enum Event not found in {EVENTS.name}")
    body = match.group("body")
    if not body.strip():
        raise SystemExit("enum Event is empty")
    variants = []
    for line in body.splitlines():
        if not line.strip() or line.strip().startswith(ALLOWED_NON_VARIANT):
            continue
        found = TUPLE_VARIANT.match(line)
        if not found:
            raise SystemExit(f"unsupported declaration in enum Event: {line.strip()!r}")
        variants.append((found.group("name"), found.group("payload")))
    if not variants:
        raise SystemExit("enum Event has no tuple variants")
    return variants


def enum_variants(text: str, name: str) -> list[str]:
    match = re.search(rf"pub enum {name} \{{(.*?)\n\}}", text, re.S)
    if not match:
        raise SystemExit(f"enum {name} not found in {PAYLOAD.name}")
    return re.findall(r"^\s{4}([A-Z]\w*),", match.group(1), re.M)


def mirror(name: str, fields: list[tuple[str, str]]) -> list[str]:
    lines = [f"#[flutter_rust_bridge::frb(mirror({name}))]"]
    if fields:
        lines.append(f"pub struct _{name} {{")
        lines.extend(f"    pub {ident}: {ty}," for ident, ty in fields)
        lines.append("}")
    else:
        lines.append(f"pub struct _{name} {{}}")
    return lines


def generate() -> str:
    payload_src = PAYLOAD.read_text()
    # Shared data types first (payload/library), then every bus message: a field
    # whose type is not mirrored makes its own mirror opaque, so the data types a
    # message refers to must be mirrors too.
    payloads = (
        payload_structs(payload_src)
        + payload_structs(LIBRARY.read_text())
        + payload_structs(MESSAGES.read_text())
    )
    variants = event_variants(EVENTS.read_text())
    if not payloads:
        raise SystemExit(f"no payload structs parsed from {MESSAGES.name}")

    structs = {name: fields for name, fields in payloads}
    seen: dict[str, str] = {}
    for name, payload in variants:
        if name != payload:
            raise SystemExit(f"event {name} wraps {payload}: name and payload differ")
        if payload not in structs:
            raise SystemExit(f"event {name} wraps unknown payload {payload}")
        if name in seen:
            raise SystemExit(f"event {name} is mirrored twice")
        seen[name] = payload

    lines = [
        "//! Generated by `scripts/gen-event-mirrors.py` — do not edit by hand.",
        "//!",
        "//! Mirrors of every `bus-api` type that crosses FRB. Without these the",
        "//! bus `Event` reaches Dart as an opaque handle instead of a sealed class.",
        "",
        "#![allow(clippy::wildcard_imports)] // generated: names come from bus-api",
        "",
        "pub use bus_api::*;",
        "",
    ]

    # Data types first: the event mirror refers to them.
    for name, fields in payloads:
        lines.extend(mirror(name, fields))
        lines.append("")

    for enum_name in PAYLOAD_ENUMS:
        lines.append(f"#[flutter_rust_bridge::frb(mirror({enum_name}))]")
        lines.append(f"pub enum _{enum_name} {{")
        lines.extend(f"    {variant}," for variant in enum_variants(payload_src, enum_name))
        lines.append("}")
        lines.append("")

    lines.append("#[flutter_rust_bridge::frb(mirror(CrossfaderAssign))]")
    lines.append("pub enum _CrossfaderAssign {")
    lines.extend(f"    {v}," for v in ["A", "Thru", "B"])
    lines.append("}")
    lines.append("")

    lines.append("#[flutter_rust_bridge::frb(mirror(Event))]")
    lines.append("pub enum _Event {")
    lines.extend(f"    {name}({payload})," for name, payload in variants)
    lines.append("}")
    lines.append("")
    return "\n".join(lines)


def display(path: Path) -> str:
    """A stable name for logs, even for a path outside the repository."""
    try:
        return str(path.relative_to(CRATES))
    except ValueError:
        return str(path)


def main(argv: list[str]) -> int:
    check = "--check" in argv[1:]
    text = generate()
    variant_count = len(event_variants(EVENTS.read_text()))
    if check:
        current = OUT.read_text() if OUT.exists() else ""
        if current != text:
            print(f"{display(OUT)} is stale; run gen-event-mirrors.py", file=sys.stderr)
            return 1
        print(f"{display(OUT)} is up to date — {variant_count} events")
        return 0
    OUT.write_text(text)
    print(f"wrote {display(OUT)} — {variant_count} events")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
