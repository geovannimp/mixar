package top.mixar.gui_flutter

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

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

    /**
     * Result of an `ensureAudioReadPermission` call that is waiting on the
     * system dialog. Completed from [onRequestPermissionsResult].
     */
    private var pendingAudioReadResult: MethodChannel.Result? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        // QNN's Hexagon backend loads its skel (`libQnnHtpV<arch>Skel.so`) from
        // this path; it must be set before libonnxruntime dlopens the HTP
        // backend. Harmless when no QNN libraries are packaged. Best effort:
        // a failing env write must not stop the app from starting, and QNN is
        // registered fail-silently in Rust anyway.
        runCatching {
            android.system.Os.setenv(
                ADSP_LIBRARY_PATH_ENV,
                applicationInfo.nativeLibraryDir,
                true,
            )
        }.onFailure {
            android.util.Log.w("Mixar", "ADSP_LIBRARY_PATH not set", it)
        }
        System.loadLibrary("host_flutter")
        initAndroidContext(applicationContext)
        super.onCreate(savedInstanceState)
    }

    override fun onDestroy() {
        // A destroyed activity never receives onRequestPermissionsResult, so
        // answer the Dart caller instead of leaving its future pending. This is
        // a plain "not granted yet" (not an error): the caller's retry re-asks.
        // The engine may already be torn down with this activity, hence the
        // guard around the reply.
        pendingAudioReadResult?.let { pending ->
            pendingAudioReadResult = null
            runCatching { pending.success(false) }
        }
        super.onDestroy()
    }

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, MEDIA_ACCESS_CHANNEL)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    ENSURE_AUDIO_READ_METHOD -> ensureAudioReadPermission(result)
                    else -> result.notImplemented()
                }
            }
    }

    /**
     * Grants read access to audio files other apps own, which scoped storage
     * otherwise hides from the raw path reads the Rust library scanner does
     * (`std::fs`). `READ_MEDIA_AUDIO` covers API 33+; older releases gate the
     * same files behind `READ_EXTERNAL_STORAGE`.
     */
    private fun ensureAudioReadPermission(result: MethodChannel.Result) {
        val permission =
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                Manifest.permission.READ_MEDIA_AUDIO
            } else {
                Manifest.permission.READ_EXTERNAL_STORAGE
            }
        if (checkSelfPermission(permission) == PackageManager.PERMISSION_GRANTED) {
            result.success(true)
            return
        }
        if (pendingAudioReadResult != null) {
            result.error("busy", "An audio permission request is already open.", null)
            return
        }
        pendingAudioReadResult = result
        requestPermissions(arrayOf(permission), AUDIO_READ_REQUEST)
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode != AUDIO_READ_REQUEST) {
            return
        }
        pendingAudioReadResult?.success(
            grantResults.isNotEmpty() && grantResults[0] == PackageManager.PERMISSION_GRANTED,
        )
        pendingAudioReadResult = null
    }

    private companion object {
        const val MEDIA_ACCESS_CHANNEL = "top.mixar.app/media_access"
        const val ENSURE_AUDIO_READ_METHOD = "ensureAudioReadPermission"
        const val AUDIO_READ_REQUEST = 4801

        /// Env var QNN's Hexagon backend reads to find its skel library.
        const val ADSP_LIBRARY_PATH_ENV = "ADSP_LIBRARY_PATH"
    }
}
