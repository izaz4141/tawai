import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:device_info_plus/device_info_plus.dart';
import 'package:tawai/utils/api_service.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/settings.dart';

class VersionInfo {
  final String version;
  final String tagName;
  late String? downloadUrl;
  final String releaseNotes;
  final String publishedAt;

  VersionInfo({
    required this.version,
    required this.tagName,
    this.downloadUrl,
    required this.releaseNotes,
    required this.publishedAt,
  });

  factory VersionInfo.fromJson(dynamic json) {
    return VersionInfo(
      version: json['version'] ?? json['tag_name'] ?? '',
      tagName: json['tag_name'] ?? '',
      releaseNotes: json['release_notes'] ?? '',
      publishedAt: json['published_at'] ?? '',
    );
  }
}

class SystemService {
  static final SystemService _instance = SystemService._internal();
  factory SystemService() => _instance;
  SystemService._internal();

  PackageInfo? _packageInfo;
  final _deviceInfoPlugin = DeviceInfoPlugin();
  final ffmpegVersion = ValueNotifier<String?>(null);
  final slskdVersion = ValueNotifier<String?>(null);
  final nadekodonVersion = ValueNotifier<String?>(null);
  final latestAppVersion = ValueNotifier<VersionInfo?>(null);
  final latestFfmpegVersion = ValueNotifier<VersionInfo?>(null);
  final latestSlskdVersion = ValueNotifier<VersionInfo?>(null);
  final latestNadekodonVersion = ValueNotifier<VersionInfo?>(null);

  Future<void> init() async {
    _packageInfo = await PackageInfo.fromPlatform();

    // Listen to online status to fetch versions accurately
    APIService.isOnline.addListener(_onStatusChanged);
    _onStatusChanged();
  }

  void _onStatusChanged() {
    if (APIService.isOnline.value) {
      fetchVersions();
      checkServiceAvailability();
    }
  }

  Future<void> checkServiceAvailability() async {
    try {
      if (SettingsManager.slskdUrl.value.isNotEmpty) {
        final result = await BridgeService.instance.testConnection('slskd');
        slskdVersion.value =
            result.success ? result.version?.replaceAll('"', '') : null;
      } else {
        slskdVersion.value = null;
      }
    } catch (e) {
      slskdVersion.value = null;
    }

    try {
      if (SettingsManager.nadekodonUrl.value.isNotEmpty) {
        final result =
            await BridgeService.instance.testConnection('nadekodon');
        nadekodonVersion.value = result.success ? result.version : null;
      } else {
        nadekodonVersion.value = null;
      }
    } catch (e) {
      nadekodonVersion.value = null;
    }
  }

  PackageInfo get packageInfo =>
      _packageInfo ??
      PackageInfo(
        appName: 'Unknown',
        packageName: 'Unknown',
        version: 'Unknown',
        buildNumber: 'Unknown',
      );

  void refreshVersions() {
    ffmpegVersion.value = null;
    slskdVersion.value = null;
    nadekodonVersion.value = null;
    latestAppVersion.value = null;
    latestFfmpegVersion.value = null;
    latestSlskdVersion.value = null;
    latestNadekodonVersion.value = null;

    if (APIService.isOnline.value) {
      fetchVersions();
      checkServiceAvailability();
    }
  }

  bool get ffmpegReady => ffmpegVersion.value != null;

  Future<void> fetchVersions() async {
    // Local tool versions
    ffmpegVersion.value =
        await BridgeService.instance.getCurrentVersion('ffmpeg');

    // Latest app version
    final latestApp = await BridgeService.instance.getLatestVersion(
      'izaz4141',
      'tawai',
      nightly: true,
    );
    latestAppVersion.value = latestApp.version != null
        ? VersionInfo(
            version: latestApp.version!,
            tagName: latestApp.tagName ?? '',
            releaseNotes: latestApp.releaseNotes ?? '',
            publishedAt: latestApp.publishedAt ?? '',
          )
        : null;

    // Latest tool versions
    final latestFfmpeg = await BridgeService.instance.getLatestVersion(
      'Ffmpeg',
      'Ffmpeg',
      nightly: true,
      atomic: true,
    );
    latestFfmpegVersion.value = latestFfmpeg.version != null
        ? VersionInfo(
            version: latestFfmpeg.version!,
            tagName: latestFfmpeg.tagName ?? '',
            releaseNotes: latestFfmpeg.releaseNotes ?? '',
            publishedAt: latestFfmpeg.publishedAt ?? '',
          )
        : null;
    final latestSlskd = await BridgeService.instance.getLatestVersion(
      'slskd',
      'slskd',
      nightly: true,
      atomic: true,
    );
    latestSlskdVersion.value = latestSlskd.version != null
        ? VersionInfo(
            version: latestSlskd.version!,
            tagName: latestSlskd.tagName ?? '',
            releaseNotes: latestSlskd.releaseNotes ?? '',
            publishedAt: latestSlskd.publishedAt ?? '',
          )
        : null;
    final latestNadekodon = await BridgeService.instance.getLatestVersion(
      'izaz4141',
      'nadekodon-rs',
      nightly: true,
      atomic: true,
    );
    latestNadekodonVersion.value = latestNadekodon.version != null
        ? VersionInfo(
            version: latestNadekodon.version!,
            tagName: latestNadekodon.tagName ?? '',
            releaseNotes: latestNadekodon.releaseNotes ?? '',
            publishedAt: latestNadekodon.publishedAt ?? '',
          )
        : null;
  }

  String? get serverVersion => APIService.serverVersion.value;

  Future<String?> getServerVersion() => APIService.instance.getServerVersion();

  String get versionString =>
      '${packageInfo.version}+${packageInfo.buildNumber}';

  Future<Map<String, String>> getDeviceInfo() async {
    if (kIsWeb) {
      final info = await _deviceInfoPlugin.webBrowserInfo;
      return {
        'Browser': info.browserName.name,
        'Platform': info.platform ?? 'Unknown',
        'User Agent': info.userAgent ?? 'Unknown',
      };
    }

    if (Platform.isAndroid) {
      final info = await _deviceInfoPlugin.androidInfo;
      return {
        'Device': '${info.brand} ${info.model}',
        'OS': 'Android ${info.version.release} (SDK ${info.version.sdkInt})',
        'ID': info.id,
      };
    } else if (Platform.isLinux) {
      final info = await _deviceInfoPlugin.linuxInfo;
      return {
        'Device': Platform.localHostname,
        'OS': '${info.prettyName} (${info.versionId})',
        'ID': info.machineId ?? 'Unknown',
      };
    } else if (Platform.isWindows) {
      final info = await _deviceInfoPlugin.windowsInfo;
      return {
        'Device': info.computerName,
        'OS':
            'Windows ${info.majorVersion}.${info.minorVersion} (Build ${info.buildNumber})',
        'ID': info.deviceId,
      };
    } else if (Platform.isMacOS) {
      final info = await _deviceInfoPlugin.macOsInfo;
      return {
        'Device': info.computerName,
        'OS': 'macOS ${info.majorVersion}.${info.minorVersion}',
        'ID': info.systemGUID ?? 'Unknown',
      };
    }

    return {'OS': kIsWeb ? 'Web' : Platform.operatingSystem};
  }

  int get processorCount => kIsWeb ? 1 : Platform.numberOfProcessors;
}
