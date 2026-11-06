package top.mixar.gui_flutter

import android.content.Context
import android.os.Bundle
import io.flutter.embedding.android.FlutterActivity

class MainActivity : FlutterActivity() {
    /**
     * Hands the JavaVM and the Application context to the Rust host.
     *
     * midir's Android MIDI backend and cpal's AAudio host resolve the JNI
     * context through `ndk-context`, which nothing in a Flutter app initializes:
     * there is no `android_activity` glue crate on this side. Without it the
     * first MIDI probe panics with "android context was not initialized" and the
     * future that awaited it never completes, so setup stays on its spinner.
     *
     * Runs before `super.onCreate`, which is where the engine starts Dart: the
     * context must be in place before any Rust host can ask for it.
     */
    private external fun initAndroidContext(context: Context)

    override fun onCreate(savedInstanceState: Bundle?) {
        System.loadLibrary("host_flutter")
        initAndroidContext(applicationContext)
        super.onCreate(savedInstanceState)
    }
}
