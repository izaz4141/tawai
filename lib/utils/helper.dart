import 'dart:io';
import 'package:flutter/foundation.dart';

import 'package:collection/collection.dart';
import 'package:device_info_plus/device_info_plus.dart';

import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/settings.dart';
import 'package:tawai/utils/io_service.dart';

String? _cachedDeviceId;

Future<String> _getDeviceId() async {
  if (_cachedDeviceId != null) return _cachedDeviceId!;

  final deviceInfo = DeviceInfoPlugin();
  String deviceId = '';

  if (Platform.isAndroid) {
    final androidInfo = await deviceInfo.androidInfo;
    deviceId = androidInfo.id;
  } else if (Platform.isLinux) {
    final linuxInfo = await deviceInfo.linuxInfo;
    deviceId = linuxInfo.id;
  } else if (Platform.isWindows) {
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

Future<bool> fileExist(String path) async {
  if (kIsWeb) return false;
  final type = await FileSystemEntity.type(path);
  return type != FileSystemEntityType.notFound;
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
  final m = (total ~/ 60).toString().padLeft(2, '0');
  final s = (total % 60).toString().padLeft(2, '0');
  return '$m:$s';
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
