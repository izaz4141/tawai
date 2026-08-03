package id.glicole.tawai

import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import com.ryanheise.audioservice.AudioServiceActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import java.io.File
import java.io.FileOutputStream

/**
 * Copies a shared audio `content://` URI into this app's cache directory so it
 * can be streamed as a plain local file by the media backend.
 */
class MainActivity : AudioServiceActivity() {
    companion object {
        private const val CHANNEL = "tawai/shared_file"
    }

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, CHANNEL)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "copyContentUri" -> {
                        val uri = call.argument<String>("uri")
                        if (uri == null) {
                            result.error("bad_args", "missing uri", null)
                        } else {
                            try {
                                result.success(copyContentUri(applicationContext, Uri.parse(uri)))
                            } catch (e: Exception) {
                                result.error("copy_failed", e.message, null)
                            }
                        }
                    }
                    else -> result.notImplemented()
                }
            }
    }

    private fun copyContentUri(context: Context, uri: Uri): String {
        val displayName = queryDisplayName(context, uri)
            ?: "shared_${System.currentTimeMillis()}"
        val target = File(context.cacheDir, displayName)
        val input = context.contentResolver.openInputStream(uri)
            ?: throw Exception("cannot open $uri")
        input.use { stream ->
            FileOutputStream(target).use { out -> stream.copyTo(out) }
        }
        return target.absolutePath
    }

    private fun queryDisplayName(context: Context, uri: Uri): String? {
        var name: String? = null
        context.contentResolver.query(uri, null, null, null, null)?.use { cursor ->
            val index = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
            if (index >= 0 && cursor.moveToFirst()) {
                name = cursor.getString(index)
            }
        }
        return name
    }
}
