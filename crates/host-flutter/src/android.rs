//! Android glue the Flutter embedding does not provide.
//!
//! `ndk-context` is normally initialized by `android_activity` in a
//! NativeActivity app. A Flutter app has no such glue — the engine lives on the
//! Java side — so every JNI-backed crate finds no context: midir's Android MIDI
//! backend and cpal's AAudio host both panic with "android context was not
//! initialized" on their first call, and a panic crossing the FRB bridge leaves
//! the Dart future pending, so first-run setup never leaves its spinner.
//!
//! `MainActivity.onCreate` loads this library and calls
//! [`Java_top_mixar_gui_1flutter_MainActivity_initAndroidContext`] before the
//! engine starts Dart, which stores the JavaVM and the Application context.
//! [`crate::jni_context`] owns the attempt rules.

use std::ffi::c_void;
use std::sync::Mutex;

use jni::objects::JObject;
use jni::JNIEnv;

use crate::jni_context::{self, ContextInit, ContextState, Report};

/// Attempt bookkeeping, under one lock because `ndk-context` asserts it is
/// initialized exactly once: after a success no later call may reach it.
static STATE: Mutex<ContextState> = Mutex::new(ContextState::new());

/// Stores the JavaVM and `context` (the Application context) for `ndk-context`.
///
/// The name is JNI-mangled: `_` in a package segment becomes `_1`, so
/// `top.mixar.gui_flutter.MainActivity.initAndroidContext` is
/// `Java_top_mixar_gui_1flutter_MainActivity_initAndroidContext`. Renaming the
/// Kotlin method or the package means renaming this function too.
///
/// # Safety
///
/// Only the Android runtime calls this: it is resolved by name from
/// `MainActivity`'s `external fun initAndroidContext(context: Context)`. The
/// `JNIEnv` and both objects must be the ones the JVM passed to that call.
#[no_mangle]
pub extern "system" fn Java_top_mixar_gui_1flutter_MainActivity_initAndroidContext<'local>(
    env: JNIEnv<'local>,
    _activity: JObject<'local>,
    context: JObject<'local>,
) {
    // A failed attempt leaves the state open, so a recreated activity tries
    // again instead of leaving the process without a context for good.
    let Ok(mut state) = STATE.lock() else {
        return;
    };
    if !state.needs_init() {
        return;
    }
    let outcome = initialize(&env, &context);
    state.record(outcome);
}

/// Pins the context and hands it to `ndk-context`.
fn initialize(env: &JNIEnv<'_>, context: &JObject<'_>) -> ContextInit {
    let vm = match env.get_java_vm() {
        Ok(vm) => vm,
        Err(_) => return ContextInit::Failed("the JVM could not be resolved"),
    };
    // A global reference: `ndk-context` keeps this jobject past the call and
    // releases nothing, so it has to outlive the local frame. The Application
    // context lives as long as the process, so it is never freed.
    let global_context = match env.new_global_ref(context) {
        Ok(global_context) => global_context,
        Err(_) => return ContextInit::Failed("the context could not be pinned"),
    };
    let context_ptr = global_context.as_obj().as_raw() as *mut c_void;
    std::mem::forget(global_context);
    // Safety: both pointers stay valid for the rest of the process, and the
    // caller holds `STATE` with `needs_init()` true, so this is the single
    // initialization the crate requires — a later call returns before here.
    unsafe {
        ndk_context::initialize_android_context(
            vm.get_java_vm_pointer() as *mut c_void,
            context_ptr,
        )
    };
    ContextInit::Initialized
}

/// Writes the recorded outcome to the application log.
///
/// Android only: called from `set_app_support_dir`, which is the point where
/// `mixar.log` starts existing. Which line to write is decided — and tested —
/// by [`crate::jni_context::report`]; this only emits it.
pub fn log_init_outcome() {
    let outcome = STATE.lock().ok().and_then(|state| state.outcome());
    match jni_context::report(outcome) {
        Report::Initialized => tracing::info!("android JNI context initialized"),
        Report::Missing(reason) => tracing::error!(
            reason,
            "android JNI context missing: MIDI and audio hosts panic on first use"
        ),
        Report::NeverAttempted => tracing::error!(
            "android JNI context was never initialized: MainActivity did not reach \
             initAndroidContext, so MIDI and audio hosts panic on first use"
        ),
    }
}
