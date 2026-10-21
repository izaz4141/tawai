import 'dart:async';
import 'dart:convert';
import 'dart:js_interop';
import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:web/web.dart' as html;
import 'io_service_base.dart';
import 'package:tawai/ui/widgets/dialog/folder_picker_dialog.dart';

class WasmIOService implements IOService {
  @override
  Future<String> getConfigDir() async {
    throw UnsupportedError(
      'Filesystem access is not supported in WASM. Use APIService for settings.',
    );
  }

  @override
  Future<String> getDownloadsDir() async {
    throw UnsupportedError(
      'Filesystem access is not supported in WASM. Use APIService for settings.',
    );
  }

  @override
  Future<String> getDatabasePath() async {
    throw UnsupportedError('Filesystem access is not supported in WASM.');
  }

  @override
  Future<String> getTorrentPersistencePath() async {
    throw UnsupportedError('Filesystem access is not supported in WASM.');
  }

  @override
  Future<bool> fileExists(String path) async {
    throw UnsupportedError('Filesystem access is not supported in WASM.');
  }

  @override
  Future<String> readFile(String path) async {
    throw UnsupportedError(
      'Filesystem access is not supported in WASM. Use APIService for settings.',
    );
  }

  @override
  Future<void> writeFile(String path, String content) async {
    throw UnsupportedError(
      'Filesystem access is not supported in WASM. Use APIService for settings.',
    );
  }

  @override
  Future<void> createDirectory(String path, {bool recursive = false}) async {
    throw UnsupportedError('Filesystem access is not supported in WASM.');
  }

  @override
  Future<bool> directoryExists(String path) async {
    throw UnsupportedError('Filesystem access is not supported in WASM.');
  }

  @override
  Future<Uint8List> readFileBytes(String path) async {
    throw UnsupportedError('Filesystem access is not supported in WASM.');
  }

  @override
  Future<void> writeFileBytes(String path, Uint8List bytes) async {
    throw UnsupportedError('Filesystem access is not supported in WASM.');
  }

  @override
  Future<String?> getDirectoryPath(
    BuildContext context, {
    String? initialPath,
  }) {
    return FolderPickerDialog.show(context, startPath: initialPath);
  }

  @override
  Future<void> setPermissions(String path, String mode) async {}

  @override
  Future<String> getTempDir() async {
    throw UnsupportedError('Filesystem access is not supported in WASM.');
  }

  @override
  Future<PickFileResult?> pickFile({List<String>? allowedExtensions}) async {
    final completer = Completer<PickFileResult?>();
    final input = html.HTMLInputElement();
    input.type = 'file';
    if (allowedExtensions != null) {
      input.accept = allowedExtensions.map((e) => '.$e').join(',');
    }
    input.onChange.listen((_) {
      final file = input.files?.item(0);
      if (file == null) {
        completer.complete(null);
        input.remove();
        return;
      }
      final reader = html.FileReader();
      reader.addEventListener(
        'load',
        ((html.Event event) {
          final dataUrl = reader.result.dartify() as String?;
          if (dataUrl == null) {
            completer.complete(null);
          } else {
            final comma = dataUrl.indexOf(',');
            final base64 = comma >= 0 ? dataUrl.substring(comma + 1) : dataUrl;
            completer.complete(
              PickFileResult(bytes: base64Decode(base64), name: file.name),
            );
          }
          input.remove();
        }).toJS,
      );
      reader.readAsDataURL(file);
    });
    input.click();
    return completer.future;
  }

  @override
  Future<void> downloadBytes(String name, Uint8List bytes) async {
    final blob = html.Blob(
      [bytes.toJS].toJS,
      html.BlobPropertyBag(type: 'application/octet-stream'),
    );
    final url = html.URL.createObjectURL(blob);
    final anchor = html.HTMLAnchorElement()
      ..href = url
      ..download = name;
    html.document.body?.appendChild(anchor);
    anchor.click();
    anchor.remove();
    html.URL.revokeObjectURL(url);
  }

  @override
  String? getCookie(String name) {
    final String rawCookies = html.window.document.cookie;
    if (rawCookies.isEmpty) return null;

    final List<String> cookies = rawCookies.split(';');

    for (final cookie in cookies) {
      final List<String> pair = cookie.split('=');
      if (pair.length == 2 && pair[0].trim() == name) {
        return pair[1].trim();
      }
    }
    return null;
  }

  @override
  void writeLine(String line) {
    // ignore: avoid_print
    print(line);
  }

  /// Prefix of the browser entries holding this device's settings.
  static const String _localSettingsPrefix = 'tawai.settings.';

  @override
  Future<Map<String, dynamic>> readLocalSettings() async {
    final result = <String, dynamic>{};
    try {
      final storage = html.window.localStorage;
      for (var i = 0; i < storage.length; i++) {
        final name = storage.key(i);
        if (name == null || !name.startsWith(_localSettingsPrefix)) continue;
        final raw = storage.getItem(name);
        if (raw == null) continue;
        try {
          result[name.substring(_localSettingsPrefix.length)] = jsonDecode(raw);
        } catch (_) {
          // One unreadable entry must not lose the rest.
        }
      }
    } catch (_) {
      // localStorage can be missing or blocked (private mode, cookie
      // settings, quota). Degrade to "no local settings".
      return <String, dynamic>{};
    }
    return result;
  }

  @override
  Future<bool> writeLocalSettings(Map<String, dynamic> values) async {
    if (values.isEmpty) return true;
    var failed = false;
    try {
      final storage = html.window.localStorage;
      for (final entry in values.entries) {
        try {
          storage.setItem(
            '$_localSettingsPrefix${entry.key}',
            jsonEncode(entry.value),
          );
        } catch (_) {
          // Keep the other entries; report the partial failure.
          failed = true;
        }
      }
    } catch (_) {
      return false;
    }
    return !failed;
  }

  @override
  void exit(int code) {
    throw UnsupportedError('Cannot exit the process in WASM.');
  }
}

IOService getIOService() => WasmIOService();
