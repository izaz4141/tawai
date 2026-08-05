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

  /// App-scoped temporary cache directory. Contents may be cleared by the
  /// system at any time. Not supported on web/wasm.
  Future<String> getTempDir();
}
