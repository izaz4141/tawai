import 'dart:convert';
import 'package:flutter/foundation.dart';

import 'package:collection/collection.dart';
import 'package:device_info_plus/device_info_plus.dart';

import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/platform_service.dart';
import 'package:tawai/utils/settings.dart';
import 'package:tawai/utils/io_service.dart';

String? _cachedDeviceId;

/// Decodes a JSON value into raw bytes. Accepts a JSON array of byte ints,
/// a base64-encoded string, or null.
Uint8List? decodeBytes(dynamic value) {
  if (value == null) return null;
  if (value is String) return base64Decode(value);
  if (value is List) return Uint8List.fromList(value.cast<int>());
  return null;
}

Future<String> _getDeviceId() async {
  if (_cachedDeviceId != null) return _cachedDeviceId!;

  final deviceInfo = DeviceInfoPlugin();
  String deviceId = '';

  if (PlatformService.isAndroid) {
    final androidInfo = await deviceInfo.androidInfo;
    deviceId = androidInfo.id;
  } else if (PlatformService.isLinux) {
    final linuxInfo = await deviceInfo.linuxInfo;
    deviceId = linuxInfo.id;
  } else if (PlatformService.isWindows) {
    final windowsInfo = await deviceInfo.windowsInfo;
    deviceId = windowsInfo.deviceId;
  }

  _cachedDeviceId = deviceId;
  return deviceId;
}

Future<String> x0(String input) async {
  final deviceId = await _getDeviceId();
  return BridgeService.instance.createTagging(input, deviceId);
}

Future<String> d0(String input) async {
  final deviceId = await _getDeviceId();
  return BridgeService.instance.putTagging(input, deviceId);
}

String formatBytes(int bytes) {
  const suffixes = ["B", "KB", "MB", "GB"];
  double size = bytes.toDouble();
  int i = 0;
  while (size >= 1024 && i < suffixes.length - 1) {
    size /= 1024;
    i++;
  }
  return "${size.toStringAsFixed(1)} ${suffixes[i]}";
}

String snakeToCamel(String input) {
  return input.split('_').mapIndexed((i, word) {
    if (i == 0) return word;
    return word[0].toUpperCase() + word.substring(1);
  }).join();
}

String pathStyling(String path) {
  final p = path.replaceAll('\\', '/');
  final posix = RegExp(r'^/home/[^/]+(?=[/]|$)');
  final win = RegExp(r'^[a-zA-Z]:/(?:.*/)?Users/[^/]+', caseSensitive: false);
  for (final re in [posix, win]) {
    final m = re.firstMatch(p);
    if (m != null) return '~${p.substring(m.end)}';
  }
  return path;
}

String camelToSnake(String input) {
  return input
      .replaceAllMapped(
        RegExp(r'([a-z0-9])([A-Z])'),
        (match) => '${match[1]}_${match[2]!.toLowerCase()}',
      )
      .toLowerCase();
}

/// The file name of [path], with no directory part. Treats `\` as a separator
/// so Windows paths work without normalising them first.
String basename(String path) => path.replaceAll('\\', '/').split('/').last;

/// The lowercase extension of [path], including the leading dot, or an empty
/// string when it has none. A leading dot marks a dotfile, not an extension.
String extension(String path) {
  final name = basename(path);
  final dot = name.lastIndexOf('.');
  if (dot <= 0) return '';
  return name.substring(dot).toLowerCase();
}

bool isLocalHost(String host) {
  final h = host.trim().toLowerCase();
  return h == '127.0.0.1' || h == 'localhost';
}

bool isUrl(String url) {
  if (url.toLowerCase().startsWith('magnet:?')) return true;
  final regex = RegExp(
    r'^(?:http|https)://'
    r'(?:(?:[A-Z0-9](?:[A-Z0-9-]{0,61}[A-Z0-9])?\.)+[A-Z]{2,6}\.?|'
    r'localhost|'
    r'\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})'
    r'(?::\d+)?'
    r'(?:/?|[/?]\S+)$',
    caseSensitive: false,
  );
  return regex.hasMatch(url);
}

bool isValidMasterKey(String key) {
  if (key.length != 64) return false;
  for (int i = 0; i < 64; i += 2) {
    final byteStr = key.substring(i, i + 2);
    if (int.tryParse(byteStr, radix: 16) == null) {
      return false;
    }
  }
  return true;
}

Future<String?> getMasterKey() async {
  if (kIsWeb) return null;
  final ioService = IOServiceFactory.create();
  final configDir = await ioService.getConfigDir();
  final masterKeyPath = '$configDir/${SettingsManager.masterKeyFile}';
  if (await ioService.fileExists(masterKeyPath)) {
    final encoded = await ioService.readFile(masterKeyPath);
    return await d0(encoded);
  }
  final key = await BridgeService.instance.generateMasterKey();
  if (key == null) return null;
  final encodedKey = await x0(key);
  await ioService.writeFile(masterKeyPath, encodedKey);
  await ioService.setPermissions(masterKeyPath, '0600');
  return key;
}

String formatFileSize(int bytes) {
  if (bytes < 1024) return '$bytes B';
  if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
  if (bytes < 1024 * 1024 * 1024) {
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
  }
  return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(1)} GB';
}

String formatSpeed(double bytesPerSec) {
  if (bytesPerSec < 1024) return '${bytesPerSec.toStringAsFixed(1)} B/s';
  if (bytesPerSec < 1024 * 1024) {
    return '${(bytesPerSec / 1024).toStringAsFixed(1)} KB/s';
  }
  return '${(bytesPerSec / (1024 * 1024)).toStringAsFixed(1)} MB/s';
}

String formatDuration(double? secs) {
  if (secs == null) return '--:--';
  final total = secs.round();

  final h = total ~/ 3600;
  final m = (total % 3600) ~/ 60;
  final s = total % 60;

  final hStr = h.toString().padLeft(2, '0');
  final mStr = m.toString().padLeft(2, '0');
  final sStr = s.toString().padLeft(2, '0');

  if (h >= 1) {
    return '$hStr:$mStr:$sStr';
  } else {
    return '$mStr:$sStr';
  }
}

String formatTimestamp(String iso) {
  try {
    final dt = DateTime.parse(iso);
    final now = DateTime.now();
    final diff = now.difference(dt);
    if (diff.inMinutes < 1) return 'Just now';
    if (diff.inMinutes < 60) return '${diff.inMinutes}m ago';
    if (diff.inHours < 24) return '${diff.inHours}h ago';
    if (diff.inDays < 7) return '${diff.inDays}d ago';
    return '${dt.month}/${dt.day}';
  } catch (_) {
    return iso;
  }
}

/// Searches XDG application directories for a Tawai `.desktop` file.
/// Returns the basename **without** the `.desktop` extension
Future<String?> findTawaiDesktopEntry() async {
  final ioService = IOServiceFactory.create();
  final home = ioService.getEnv('HOME');
  if (home == null) return null;

  final dataHome = ioService.getEnv('XDG_DATA_HOME') ?? '$home/.local/share';
  final dataDirs =
      ioService.getEnv('XDG_DATA_DIRS') ?? '/usr/local/share:/usr/share';

  // Search order matters: user-installed files should win over system ones.
  final searchDirs = <String>[
    '$dataHome/applications',
    for (final dir in dataDirs.split(':'))
      if (dir.isNotEmpty) '$dir/applications',
  ];

  const exactNames = <String>[
    'tawai.desktop',
    'Tawai.desktop',
    'TAWAI.desktop',
    'id.glicole.tawai.desktop',
    'Tawai-AM.desktop', // AppMan
    'tawai-AM.desktop',
  ];

  for (final dir in searchDirs) {
    for (final fileName in exactNames) {
      if (await ioService.fileExists('$dir/$fileName')) {
        return fileName.substring(0, fileName.length - '.desktop'.length);
      }
    }
  }
  return null;
}
