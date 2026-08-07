//! Optional Rhai host for mapping hooks and script bindings.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use engine_api::{CmdBody, Kind, Origin};
use rhai::{Dynamic, Engine, Module, Scope, AST};

use crate::error::{LoadError, RuntimeError};
use crate::session::{ActionPublish, MidiOut};

/// Shared bridge used by registered Rhai functions.
pub struct ScriptBridge {
    pub published: Vec<(Origin, Kind, CmdBody)>,
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

        let bridge: Arc<Mutex<ScriptScratch>> = Arc::new(Mutex::new(ScriptScratch {
            publish_queue: Vec::new(),
            midi_queue: Vec::new(),
            modifiers: Vec::new(),
        }));

        let mut module = Module::new();
        {
            let b = Arc::clone(&bridge);
            module.set_native_fn("midi_out", move |bytes: Vec<Dynamic>| {
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
            module.set_native_fn(
                "publish",
                move |origin: &str, kind: &str, _payload: &str| {
                    if let Ok(mut g) = b.lock() {
                        g.publish_queue
                            .push((origin.to_string(), kind.to_string(), String::new()));
                    }
                    Ok::<(), Box<rhai::EvalAltResult>>(())
                },
            );
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
            bridge,
        })
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
        for (origin_s, kind_s, _) in pubs {
            // Scripts pass flat strings (`engine`, `deck1`); wire Origin for Deck is
            // `{"deck":n}`, so serde alone isn't enough for the Rhai publish API.
            let origin = parse_origin(&origin_s).unwrap_or(Origin::Engine);
            let kind = parse_kind(&kind_s).unwrap_or(Kind::Notice);
            host.bus.publish_engine(origin, kind, CmdBody::Empty);
        }
        for m in midis {
            host.midi.send(&m);
        }
    }

    pub fn has_fn(&self, name: &str) -> bool {
        self.ast.iter_functions().any(|f| f.name == name)
    }

    pub fn call_hook(&mut self, name: &str, host: &mut ScriptHost<'_>) -> Result<(), RuntimeError> {
        self.prepare_scratch(host);
        let mut scope = Scope::new();
        // Hooks take no args in v1; ctx is implicit via registered fns.
        let result = self.engine.call_fn::<()>(&mut scope, &self.ast, name, ());
        self.flush_scratch(host);
        match result {
            Ok(()) => Ok(()),
            Err(e) => {
                // Missing optional hook is fine.
                let msg = e.to_string();
                if msg.contains("not found") || msg.contains("Function not found") {
                    Ok(())
                } else {
                    Err(RuntimeError::Script(msg))
                }
            }
        }
    }

    pub fn call_named(
        &mut self,
        name: &str,
        host: &mut ScriptHost<'_>,
        value_01: f32,
        active: bool,
    ) -> Result<(), RuntimeError> {
        self.prepare_scratch(host);
        let mut scope = Scope::new();
        let result =
            self.engine
                .call_fn::<()>(&mut scope, &self.ast, name, (value_01 as f64, active));
        self.flush_scratch(host);
        result.map_err(|e| RuntimeError::Script(e.to_string()))
    }
}

fn parse_origin(s: &str) -> Option<Origin> {
    match s {
        "engine" => Some(Origin::Engine),
        "mixer" => Some(Origin::Mixer),
        s if s.starts_with("deck") => {
            let n: u16 = s.strip_prefix("deck").unwrap_or("0").parse().ok()?;
            Some(Origin::Deck(n.saturating_sub(1).min(3)))
        }
        _ => None,
    }
}

fn parse_kind(s: &str) -> Option<Kind> {
    match s {
        "play" => Some(Kind::Play),
        "pause" => Some(Kind::Pause),
        "set_volume" => Some(Kind::SetVolume),
        "notice" => Some(Kind::Notice),
        "error" => Some(Kind::Error),
        _ => None,
    }
}
