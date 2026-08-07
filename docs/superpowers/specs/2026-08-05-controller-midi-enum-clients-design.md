# Controller MIDI enumeration clients

**Date:** 2026-08-05  
**Status:** accepted

## Goal

One long-lived midir input + output used only for port enumeration; each mapping attach still opens its own connect clients (path to multi-controller).

## Decision

| Decision | Choice |
|----------|--------|
| Enum clients | Lazy `enum_in` / `enum_out` on `ControllerEngine` (first list/poll); never `connect`ed |
| Connect | Fresh `MidiInput::new` / `MidiOutput::new` per `enable_mapping` (midir consumes on `connect`) |
| Poll API | `ControllerEngine::list_input_port_names` / `poll_devices`; free function removed |
| Multi-controller | Out of scope; `Option<Attached>` stays; N attaches = N connect pairs later |

## Out of scope

`Vec<Attached>`, sharing one connection across mappings.
