//! Optional Rhai host for mapping hooks and script bindings.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use engine_api::Error;
use engine_api::Event;
use engine_api::Notice;
use engine_api::Pause;
use engine_api::Play;
use engine_api::SetVolume;
use rhai::{CallFnOptions, Dynamic, Engine, Module, Scope, AST};

use crate::action::EngineScope;
use crate::error::{LoadError, RuntimeError};
use crate::session::{ActionPublish, MidiOut};

/// shortcut: caps mapping script CPU; raise if legitimate bundles hit it.
const MAX_SCRIPT_OPERATIONS: u64 = 50_000;
const MAX_MIDI_OUT_BYTES: usize = 256;

/// Shared bridge used by registered Rhai functions.
pub struct ScriptBridge {
    pub published: Vec<Event>,
    pub midi: Vec<Vec<u8>>,
}

/// Host handles passed into a script call.
pub struct ScriptHost<'a> {
    pub bus: &'a mut dyn ActionPublish,
    pub midi: &'a mut dyn MidiOut,
    pub modifiers: &'a HashSet<String>,
}

pub struct ScriptRuntime {
    engine: Engine,
    ast: AST,
    /// Holds top-level `const` / `let` after one AST eval (Rhai's default `call_fn`
    /// rewinds the scope and drops those bindings before the hook body runs).
    scope: Scope<'static>,
    globals_ready: bool,
    /// Scratch bridge for sync callbacks during a call.
    bridge: Arc<Mutex<ScriptScratch>>,
}

struct ScriptScratch {
    publish_queue: Vec<(String, String, String)>,
    midi_queue: Vec<Vec<u8>>,
    modifiers: Vec<String>,
}

impl ScriptRuntime {
    pub fn compile(source: &str) -> Result<Self, LoadError> {
        let mut engine = Engine::new();
        // Keep std + sync; disable time-heavy defaults if any.
        engine.set_max_expr_depths(64, 32);
        engine.set_max_operations(MAX_SCRIPT_OPERATIONS);

        let bridge: Arc<Mutex<ScriptScratch>> = Arc::new(Mutex::new(ScriptScratch {
            publish_queue: Vec::new(),
            midi_queue: Vec::new(),
            modifiers: Vec::new(),
        }));

        let mut module = Module::new();
        {
            let b = Arc::clone(&bridge);
            module.set_native_fn("midi_out", move |bytes: Vec<Dynamic>| {
                if bytes.len() > MAX_MIDI_OUT_BYTES {
                    return Err("midi_out payload too large".into());
                }
                let mut raw = Vec::with_capacity(bytes.len());
                for v in bytes {
                    if let Ok(n) = v.as_int() {
                        raw.push(n as u8);
                    }
                }
                if let Ok(mut g) = b.lock() {
                    g.midi_queue.push(raw);
                }
                Ok::<(), Box<rhai::EvalAltResult>>(())
            });
        }
        {
            let b = Arc::clone(&bridge);
            module.set_native_fn("publish", move |origin: &str, kind: &str, payload: &str| {
                if let Ok(mut g) = b.lock() {
                    g.publish_queue.push((
                        origin.to_string(),
                        kind.to_string(),
                        payload.to_string(),
                    ));
                }
                Ok::<(), Box<rhai::EvalAltResult>>(())
            });
        }
        {
            let b = Arc::clone(&bridge);
            module.set_native_fn("modifier_active", move |name: &str| {
                Ok::<_, Box<rhai::EvalAltResult>>(
                    b.lock()
                        .map(|g| g.modifiers.iter().any(|m| m == name))
                        .unwrap_or(false),
                )
            });
        }
        // Global module keeps flat call sites (`midi_out(...)`) while grouping host fns.
        engine.register_global_module(module.into());

        let ast = engine
            .compile(source)
            .map_err(|e| LoadError::ScriptCompile(e.to_string()))?;

        Ok(Self {
            engine,
            ast,
            scope: Scope::new(),
            globals_ready: false,
            bridge,
        })
    }

    fn ensure_globals(&mut self) -> Result<(), RuntimeError> {
        if self.globals_ready {
            return Ok(());
        }
        self.engine
            .run_ast_with_scope(&mut self.scope, &self.ast)
            .map_err(|e| RuntimeError::Script(e.to_string()))?;
        self.globals_ready = true;
        Ok(())
    }

    fn call_script_fn(
        &mut self,
        name: &str,
        args: impl rhai::FuncArgs,
    ) -> Result<(), RuntimeError> {
        self.ensure_globals()?;
        // Globals already in `scope`; rewind function-locals after the call.
        let options = CallFnOptions::new().eval_ast(false).rewind_scope(true);
        self.engine
            .call_fn_with_options::<()>(options, &mut self.scope, &self.ast, name, args)
            .map_err(|e| RuntimeError::Script(e.to_string()))
    }

    fn prepare_scratch(&self, host: &ScriptHost<'_>) {
        if let Ok(mut g) = self.bridge.lock() {
            g.publish_queue.clear();
            g.midi_queue.clear();
            g.modifiers = host.modifiers.iter().cloned().collect();
        }
    }

    fn flush_scratch(&self, host: &mut ScriptHost<'_>) {
        let (pubs, midis) = if let Ok(mut g) = self.bridge.lock() {
            (
                std::mem::take(&mut g.publish_queue),
                std::mem::take(&mut g.midi_queue),
            )
        } else {
            return;
        };
        for (origin_s, kind_s, payload) in pubs {
            // Scripts pass flat strings (`engine`, `deck1`); the bus carries the
            // deck inside each payload, so the host resolves the scope here.
            let scope = parse_scope(&origin_s).unwrap_or(EngineScope::Engine);
            let Some(event) = resolve_publish(scope, &kind_s, &payload) else {
                continue;
            };
            host.bus.publish_engine(event);
        }
        for m in midis {
            host.midi.send(&m);
        }
    }

    pub fn has_fn(&self, name: &str) -> bool {
        self.ast.iter_functions().any(|f| f.name == name)
    }

    pub fn call_hook(&mut self, name: &str, host: &mut ScriptHost<'_>) -> Result<(), RuntimeError> {
        // Missing optional hook is fine; do not swallow VariableNotFound etc.
        if !self.has_fn(name) {
            return Ok(());
        }
        self.prepare_scratch(host);
        // Hooks take no args in v1; ctx is implicit via registered fns.
        let result = self.call_script_fn(name, ());
        self.flush_scratch(host);
        result
    }

    pub fn call_named(
        &mut self,
        name: &str,
        host: &mut ScriptHost<'_>,
        value_01: f32,
        active: bool,
    ) -> Result<(), RuntimeError> {
        self.prepare_scratch(host);
        let result = self.call_script_fn(name, (value_01 as f64, active));
        self.flush_scratch(host);
        result
    }
}

fn parse_scope(s: &str) -> Option<EngineScope> {
    match s {
        "engine" => Some(EngineScope::Engine),
        "mixer" => Some(EngineScope::Mixer),
        s if s.starts_with("deck") => {
            let n: u16 = s.strip_prefix("deck").unwrap_or("0").parse().ok()?;
            Some(EngineScope::Deck(n.saturating_sub(1).min(3)))
        }
        _ => None,
    }
}

/// Script `set_volume` payload: `{"type":"set_volume","volume":0.5}`.
///
/// Extra JSON keys (`type`, …) are ignored by serde.
#[derive(serde::Deserialize)]
struct SetVolumePayload {
    volume: f32,
    #[serde(default)]
    soft_takeover: bool,
}

/// Resolve one script `publish(origin, kind, payload)` call into a typed event.
///
/// `None` drops the call (`kind` outside the script surface, or a payload that
/// does not parse) after a warning, matching the previous wire behaviour.
fn resolve_publish(scope: EngineScope, kind: &str, payload: &str) -> Option<Event> {
    match kind {
        "play" => Some(Event::Play(Play {
            deck: scope.deck_or_default(),
        })),
        "pause" => Some(Event::Pause(Pause {
            deck: scope.deck_or_default(),
        })),
        "set_volume" => {
            let parsed: SetVolumePayload = match serde_json::from_str(payload) {
                Ok(parsed) => parsed,
                Err(err) => {
                    tracing::warn!("script publish payload parse failed: {err}");
                    return None;
                }
            };
            Some(Event::SetVolume(SetVolume {
                deck: scope.deck_or_default(),
                volume: parsed.volume,
                soft_takeover: parsed.soft_takeover,
            }))
        }
        "notice" => Some(Event::Notice(Notice {
            message: payload.to_string(),
        })),
        "error" => Some(Event::Error(Error {
            message: payload.to_string(),
            track_id: None,
            deck: Some(scope.deck_or_default()),
        })),
        _ => None,
    }
}
