import 'dart:async';
import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

class PickFileResult {
  final String? name;
  final Uint8List? bytes;
  final String? path;

  const PickFileResult({this.name, this.bytes, this.path});
}

abstract class IOService {
  Future<String> getConfigDir();
  Future<String> getDownloadsDir();
  Future<String> getDatabasePath();
  Future<String> getTorrentPersistencePath();
  Future<bool> fileExists(String path);
  Future<String> readFile(String path);
  Future<void> writeFile(String path, String content);
  Future<void> createDirectory(String path, {bool recursive = false});
  Future<bool> directoryExists(String path);
  Future<Uint8List> readFileBytes(String path);
  Future<void> writeFileBytes(String path, Uint8List bytes);
  Future<String?> getDirectoryPath(BuildContext context, {String? initialPath});
  Future<void> setPermissions(String path, String mode);
  String? getCookie(String name);
  Future<PickFileResult?> pickFile({List<String>? allowedExtensions});

  /// Device-local settings persisted by the platform itself.
  ///
  /// On web these live in the browser's local storage (one entry per
  /// setting); on native the device's own config file already holds them
  /// (written through Rinf), so the native implementation is a no-op.
  Future<Map<String, dynamic>> readLocalSettings();

  /// Persists [values] in the platform's device-local settings, one entry
  /// per key. Returns false when the platform cannot persist them.
  Future<bool> writeLocalSettings(Map<String, dynamic> values);

  /// App-scoped temporary cache directory. Contents may be cleared by the
  /// system at any time. Not supported on web/wasm.
  Future<String> getTempDir();

  /// Triggers a platform download of [bytes] under the name [name].
  /// Only meaningful on web/wasm; no-op on platforms that write in place.
  Future<void> downloadBytes(String name, Uint8List bytes);

  /// Writes [line] to standard output, followed by a newline.
  void writeLine(String line);

  /// Ends the process immediately with [code]. Never returns.
  void exit(int code);
}
