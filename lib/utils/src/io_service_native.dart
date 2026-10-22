import 'dart:async';
import 'dart:io';
// Prefixed as well, so [NativeIOService.exit] can reach `dart:io`'s exit
// without calling itself.
import 'dart:io' as io;
import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:synchronized/synchronized.dart';
import 'package:path_provider/path_provider.dart';
import 'package:file_picker/file_picker.dart';
import 'io_service_base.dart';
import 'package:tawai/utils/platform_service.dart';
import 'package:tawai/ui/widgets/dialog/folder_picker_dialog.dart';

class NativeIOService implements IOService {
  final Map<String, Lock> _fileLocks = {};

  Lock _getLock(String path) =>
      _fileLocks.putIfAbsent(path, () => Lock(reentrant: true));

  @override
  Future<String> getConfigDir() async {
    final dir = await getApplicationSupportDirectory();
    return dir.path;
  }

  @override
  Future<String> getDownloadsDir() async {
    final downloads = await getDownloadsDirectory();
    return downloads?.path ?? '';
  }

  @override
  Future<String> getDatabasePath() async {
    final configDir = await getConfigDir();
    return '$configDir/tawai.db';
  }

  @override
  Future<String> getTorrentPersistencePath() async {
    final configDir = await getConfigDir();
    return '$configDir/torrent_data';
  }

  @override
  Future<bool> fileExists(String path) async {
    return File(path).exists();
  }

  @override
  Future<String> readFile(String path) async {
    return await _getLock(path).synchronized(() async {
      return File(path).readAsString();
    });
  }

  @override
  Future<void> writeFile(String path, String content) async {
    await File(path).parent.create(recursive: true);
    await _getLock(path).synchronized(() async {
      await File(path).writeAsString(content);
    });
  }

  @override
  Future<void> createDirectory(String path, {bool recursive = false}) async {
    final dir = Directory(path);
    await dir.create(recursive: recursive);
  }

  @override
  Future<bool> directoryExists(String path) async {
    return Directory(path).exists();
  }

  @override
  Future<Uint8List> readFileBytes(String path) async {
    return await _getLock(path).synchronized(() async {
      return File(path).readAsBytes();
    });
  }

  @override
  Future<void> writeFileBytes(String path, Uint8List bytes) async {
    await File(path).parent.create(recursive: true);
    await _getLock(path).synchronized(() async {
      await File(path).writeAsBytes(bytes);
    });
  }

  @override
  Future<String?> getDirectoryPath(
    BuildContext context, {
    String? initialPath,
  }) async {
    if (PlatformService().isRemote) {
      return FolderPickerDialog.show(context, startPath: initialPath);
    }
    return await FilePicker.getDirectoryPath();
  }

  @override
  Future<PickFileResult?> pickFile({List<String>? allowedExtensions}) async {
    final file = await FilePicker.pickFile(
      type: allowedExtensions != null ? FileType.custom : FileType.any,
      allowedExtensions: allowedExtensions,
    );
    if (file == null) return null;
    return PickFileResult(
      path: file.path,
      bytes: await File(file.path!).readAsBytes(),
      name: file.name,
    );
  }

  @override
  Future<void> setPermissions(String path, String mode) async {
    if (!PlatformService.isLinux) return;
    await Process.run('chmod', [mode, path]);
  }

  @override
  Future<String> getTempDir() async {
    final temp = await getTemporaryDirectory();
    return temp.path;
  }

  @override
  Future<void> downloadBytes(String name, Uint8List bytes) async {}

  /// Native keeps the device-local settings in its own config file, written
  /// through Rinf (`RinfService.saveLocalConfig`), so there is nothing to
  /// keep here.
  @override
  Future<Map<String, dynamic>> readLocalSettings() async => <String, dynamic>{};

  @override
  Future<bool> writeLocalSettings(Map<String, dynamic> values) async => false;

  @override
  void writeLine(String line) => stdout.writeln(line);

  @override
  void exit(int code) => io.exit(code);

  @override
  String? getCookie(String name) {
    throw UnsupportedError('Theres no cookie in native app.');
  }
}

IOService getIOService() => NativeIOService();
