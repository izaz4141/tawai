import 'dart:async';
import 'dart:convert';
import 'dart:js_interop';
import 'package:flutter/foundation.dart';
import 'package:web/web.dart' as html;
import 'io_service_base.dart';

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
  Future<String?> getDirectoryPath() async {
    throw UnsupportedError('Directory picker is not supported in WASM.');
  }

  @override
  Future<void> setPermissions(String path, String mode) async {}

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
      reader.addEventListener('load', ((html.Event event) {
        final dataUrl = reader.result.dartify() as String?;
        if (dataUrl == null) {
          completer.complete(null);
        } else {
          final comma = dataUrl.indexOf(',');
          final base64 = comma >= 0 ? dataUrl.substring(comma + 1) : dataUrl;
          completer.complete(PickFileResult(
            bytes: base64Decode(base64),
            name: file.name,
          ));
        }
        input.remove();
      }).toJS);
      reader.readAsDataURL(file);
    });
    input.click();
    return completer.future;
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
}

IOService getIOService() => WasmIOService();
