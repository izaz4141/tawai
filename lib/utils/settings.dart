import 'dart:async';
import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'package:tawai/utils/platform_service.dart';
import 'package:tawai/utils/logger.dart';
import 'package:tawai/utils/io_service.dart';
import 'package:tawai/utils/api_service.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/rinf_service.dart';
import 'package:tawai/utils/system_service.dart';
import 'package:tawai/utils/helper.dart';
import 'package:tawai/models/account.dart';
import 'package:tawai/models/user.dart';

class SettingsManager {
  static const String masterKeyFile = 'master.key';
  static late IOService _ioService;
  static late String configPath;
  static bool isFirstRun = false;
  static Map<String, dynamic> defaults = {};

  // Your ValueNotifiers
  static final retreatToTray = ValueNotifier<bool>(true);
  static final downloadFolder = ValueNotifier<String>('');
  static final serverHost = ValueNotifier<String>('127.0.0.1');
  static final serverPort = ValueNotifier<int>(8080);
  static final currentUser = ValueNotifier<User?>(null);

  static final accounts = ValueNotifier<List<Account>>([]);
  static final Map<ValueNotifier, VoidCallback> _autoSaveListeners = {};

  // Theme Settings
  static final themeMode = ValueNotifier<ThemeMode>(ThemeMode.system);
  static final useDynamicColor = ValueNotifier<bool>(true);
  static final customColor = ValueNotifier<int>(0xFFFF4081);

  static final checkNightly = ValueNotifier<bool>(false);

  // Login Settings
  static final requireLogin = ValueNotifier<bool>(kIsWeb);
  static final isLoggedIn = ValueNotifier<bool>(false);
  static final currentUserId = ValueNotifier<String?>(null);

  // slskd Settings
  static final slskdUrl = ValueNotifier<String>('');
  static final slskdApiKey = ValueNotifier<String>('');
  static final desiredAudioQuality = ValueNotifier<String>('best');

  // nadekodon Settings
  static final nadekodonUrl = ValueNotifier<String>('');
  static final nadekodonApiKey = ValueNotifier<String>('');

  // default download source
  static final defaultDownloadSource = ValueNotifier<String>('slskd');

  // ListenBrainz Settings
  static final listenbrainzToken = ValueNotifier<String>('');

  // Recommendation sources shown in library (comma-separated keys)
  static final includedRecommendations = ValueNotifier<String>('');

  // Naming Pattern
  static final namingPattern = ValueNotifier<String>(
    '{album_artist??{artist?|/}|/}{album_artist?{album?|/}}{total_discs>1?{disc_padded}|-}{album_artist?{track_padded}| }{multi_artist?{artist}| - }{title}',
  );

  // Per-user metadata preferences
  static final lyricsPrefersync = ValueNotifier<bool>(true);

  // Per-user streaming preferences
  static final preferredBitrate = ValueNotifier<String>('lossless');

  // Per-user playback preferences
  static final replayGainEnabled = ValueNotifier<bool>(false);
  static final replayGainPreamp = ValueNotifier<double>(0.0);

  // Playback preferences
  static final playbackVolume = ValueNotifier<double>(1.0);

  static Future<void> _loadDefaults() async {
    try {
      final String response = await rootBundle.loadString(
        'assets/docs/default.json',
      );
      defaults = json.decode(response);
    } catch (e) {
      log('Error loading default settings asset: $e', isError: true);
    }
  }

  static Future<void> init() async {
    _ioService = IOServiceFactory.create();
    await _loadDefaults();

    if (kIsWeb) {
      return;
    }

    final downloadsDir = await _ioService.getDownloadsDir();
    final configDir = await _ioService.getConfigDir();

    String defaultDownloadFolder = '';
    if (PlatformService.isAndroid) {
      defaultDownloadFolder = '/storage/emulated/0/Download';
    } else {
      defaultDownloadFolder = downloadsDir;
    }

    if (defaultDownloadFolder.isNotEmpty) {
      final exists = await _ioService.directoryExists(defaultDownloadFolder);
      if (!exists) {
        await _ioService.createDirectory(
          defaultDownloadFolder,
          recursive: true,
        );
      }
    }

    configPath = '$configDir/config.json';

    final result = await RinfService.instance.initConfig(configPath);
    final data = result.settings;
    if (data != null) {
      serverHost.value =
          data['server_host'] ?? (defaults['server_host'] ?? '127.0.0.1');
      serverPort.value =
          data['server_port'] ?? (defaults['server_port'] ?? 8080);
      await _applyFromJson(data);
    }

    isFirstRun = result.isFirstRun;
    if (isFirstRun) {
      downloadFolder.value = defaultDownloadFolder;
      await saveAll();
    }

    attachAutoSave();
  }

  static const List<String> deviceOnlyKeys = [
    'playback_volume',
    'current_user',
    'require_login',
    'check_nightly',
    'accounts',
  ];

  static Future<void> _applyFromJson(
    Map<String, dynamic> json, {
    bool fromBackend = false,
  }) async {
    retreatToTray.value =
        json['retreat_to_tray'] ?? defaults['retreat_to_tray'];
    downloadFolder.value = json['download_folder'] ?? '';

    // Theme Settings
    if (json['theme_mode'] != null) {
      themeMode.value = ThemeMode.values[json['theme_mode']];
    } else if (defaults['theme_mode'] != null) {
      themeMode.value = ThemeMode.values[defaults['theme_mode']];
    }
    useDynamicColor.value =
        json['use_dynamic_color'] ?? defaults['use_dynamic_color'];
    customColor.value = json['custom_color'] ?? defaults['custom_color'];
    if (!fromBackend) {
      checkNightly.value = json['check_nightly'] ?? defaults['check_nightly'];
      if (json.containsKey('require_login')) {
        requireLogin.value = json['require_login'] ?? false;
      }
      currentUserId.value = json['current_user'] as String?;
    }

    slskdUrl.value = json['slskd_url'] ?? '';
    nadekodonUrl.value = json['nadekodon_url'] ?? '';
    desiredAudioQuality.value = json['desired_audio_quality'] ?? 'best';
    defaultDownloadSource.value = json['default_download_source'] ?? 'slskd';
    if (defaultDownloadSource.value == 'ytdlp')
      defaultDownloadSource.value = 'slskd';

    if (!fromBackend) {
      if (json['playback_volume'] is num) {
        playbackVolume.value = (json['playback_volume'] as num).toDouble();
      } else if (defaults['playback_volume'] is num) {
        playbackVolume.value = (defaults['playback_volume'] as num).toDouble();
      }

      if (json['accounts'] != null) {
        final accountList = <Account>[];
        for (final accJson in json['accounts']) {
          accountList.add(Account.fromJson(accJson));
        }
        accounts.value = accountList;
      }
    }
  }

  static Future<Map<String, dynamic>> _toJson() async => {
    'retreat_to_tray': retreatToTray.value,
    'download_folder': downloadFolder.value,
    'server_host': serverHost.value,
    'server_port': serverPort.value,
    'theme_mode': themeMode.value.index,
    'use_dynamic_color': useDynamicColor.value,
    'custom_color': customColor.value,
    'check_nightly': checkNightly.value,
    'require_login': requireLogin.value,
    'current_user': currentUserId.value,
    'slskd_url': slskdUrl.value,
    'nadekodon_url': nadekodonUrl.value,
    'desired_audio_quality': desiredAudioQuality.value,
    'default_download_source': defaultDownloadSource.value,
    'playback_volume': playbackVolume.value,
    'accounts': accounts.value.map((e) => e.toJson()).toList(),
  };

  static Future<void> reloadConfig() async {
    if (PlatformService().isRemote) return await loadFromBackend();

    final result = await RinfService.instance.initConfig(configPath);
    final data = result.settings;
    if (data != null) {
      serverHost.value =
          data['server_host'] ?? (defaults['server_host'] ?? '127.0.0.1');
      serverPort.value =
          data['server_port'] ?? (defaults['server_port'] ?? 8080);
      await _applyFromJson(data);
    }
  }

  static Future<void> saveAll() async {
    if (kIsWeb) {
      await _saveToBackend();
      return;
    }
    final jsonMap = await _toJson();
    await RinfService.instance.saveConfig(jsonMap);
  }

  static Future<void> saveChanged(String key, dynamic value) async {
    if (PlatformService().isRemote) {
      if (key != 'accounts') {
        await _saveToBackend();
      }
      return;
    }

    await RinfService.instance.saveConfig({key: value});
  }

  static void attachAutoSave() {
    detachAutoSave();

    void add(ValueNotifier n, String key, [dynamic Function()? getValue]) {
      void listener() =>
          saveChanged(key, getValue != null ? getValue() : n.value);
      n.addListener(listener);
      _autoSaveListeners[n] = listener;
    }

    add(retreatToTray, 'retreat_to_tray');
    add(downloadFolder, 'download_folder');
    add(serverHost, 'server_host');
    add(serverPort, 'server_port');
    add(themeMode, 'theme_mode', () => themeMode.value.index);
    add(useDynamicColor, 'use_dynamic_color');
    add(customColor, 'custom_color');
    add(checkNightly, 'check_nightly');
    add(requireLogin, 'require_login');
    add(slskdUrl, 'slskd_url');
    add(nadekodonUrl, 'nadekodon_url');
    add(desiredAudioQuality, 'desired_audio_quality');
    add(defaultDownloadSource, 'default_download_source');
    add(playbackVolume, 'playback_volume');
    add(namingPattern, 'naming_pattern');
    add(
      accounts,
      'accounts',
      () => accounts.value.map((e) => e.toJson()).toList(),
    );
  }

  static void detachAutoSave() {
    _autoSaveListeners.forEach((notifier, listener) {
      notifier.removeListener(listener);
    });
    _autoSaveListeners.clear();
  }

  static Future<void> applyDefaultSettings() async {
    retreatToTray.value = defaults['retreat_to_tray'] ?? true;
    // downloadFolder is usually not reset to default from asset as it's environment dependent
    serverHost.value = defaults['server_host'] ?? '127.0.0.1';
    serverPort.value = defaults['server_port'] ?? 8080;

    if (defaults['theme_mode'] != null) {
      themeMode.value = ThemeMode.values[defaults['theme_mode']];
    }
    useDynamicColor.value = defaults['use_dynamic_color'] ?? true;
    customColor.value = defaults['custom_color'] ?? 0xFFFF4081;
    checkNightly.value = defaults['check_nightly'] ?? false;
    namingPattern.value =
        defaults['naming_pattern'] ??
        '{album_artist??{artist?|/}|/}{album_artist?{album?|/}}{total_discs>1?{disc_padded}|-}{album_artist?{track_padded}| }{multi_artist?{artist}| - }{title}';
  }

  static void syncNamingPatternToRust(String pattern) {
    if (PlatformService().isRemote) return;
    final userId = currentUserId.value ?? '';
    if (userId.isEmpty) return;
    unawaited(
      BridgeService.instance.setUserSetting(
        userId,
        'identify_naming_pattern',
        pattern,
      ),
    );
  }

  static Future<void> loadAllUserSettings() async {
    final userId = currentUserId.value ?? '';
    if (userId.isEmpty) {
      return;
    }
    try {
      final settings = await BridgeService.instance.getAllUserSettings(userId);

      if (settings.containsKey('lyrics_prefersync')) {
        lyricsPrefersync.value = settings['lyrics_prefersync']! == 'true';
      }
      if (settings.containsKey('preferred_bitrate')) {
        preferredBitrate.value = settings['preferred_bitrate']!;
      }
      if (settings.containsKey('listenbrainz_token')) {
        listenbrainzToken.value = settings['listenbrainz_token']!;
      }
      if (settings.containsKey('included_recommendations')) {
        includedRecommendations.value = settings['included_recommendations']!;
      }
      if (settings.containsKey('replay_gain_enabled')) {
        replayGainEnabled.value = settings['replay_gain_enabled']! == 'true';
      }
      if (settings.containsKey('replay_gain_preamp')) {
        replayGainPreamp.value =
            double.tryParse(settings['replay_gain_preamp']!) ?? 0.0;
      }
    } catch (e) {
      // ignore
    }
  }

  static Future<void> saveUserSetting(
    ValueNotifier<dynamic> notifier,
    String key,
    dynamic value,
  ) async {
    notifier.value = value;
    final userId = currentUserId.value ?? '';
    if (userId.isEmpty) {
      return;
    }
    try {
      await BridgeService.instance.setUserSetting(
        userId,
        key,
        value.toString(),
      );
    } catch (e) {
      // ignore
    }
  }

  static Future<String> getDatabasePath() async {
    return _ioService.getDatabasePath();
  }

  static Future<String> getTorrentPersistencePath() async {
    return _ioService.getTorrentPersistencePath();
  }

  static Future<void> regenerateApiKey(String userId) async {
    if (PlatformService().isRemote) {
      await APIService.instance.regenerateApiKey();
      return;
    }
    final configDir = await _ioService.getConfigDir();
    final masterKeyPath = '$configDir/$masterKeyFile';
    final masterKeyExists = await _ioService.fileExists(masterKeyPath);
    String? existingMasterKey;
    if (masterKeyExists) {
      final encoded = await _ioService.readFile(masterKeyPath);
      existingMasterKey = await d0(encoded);
    }
    final result = await BridgeService.instance.requestNewApiKey(
      masterKey: existingMasterKey,
      userId: userId,
    );

    if (result.masterKey.isNotEmpty) {
      final encodedKey = await x0(result.masterKey);
      await _ioService.writeFile(masterKeyPath, encodedKey);
      await _ioService.setPermissions(masterKeyPath, '0600');
    }
  }

  static Future<void> saveSlskdApiKey(String plainKey) async {
    slskdApiKey.value = plainKey;
    if (plainKey.isEmpty) return;
    if (PlatformService().isRemote) {
      await APIService.instance.saveSettings({'slskd_api_key': plainKey});
      return;
    }
    await RinfService.instance.saveConfig({'slskd_api_key': plainKey});
  }

  static Future<void> saveNadekodonApiKey(String plainKey) async {
    nadekodonApiKey.value = plainKey;
    if (plainKey.isEmpty) return;
    if (PlatformService().isRemote) {
      await APIService.instance.saveSettings({'nadekodon_api_key': plainKey});
      return;
    }
    await RinfService.instance.saveConfig({'nadekodon_api_key': plainKey});
  }

  static Future<void> restartServer() async {
    if (PlatformService().isRemote) {
      await APIService.instance.restartServer();
      return;
    }
    final masterKey = await getMasterKey();
    BridgeService.instance.startServer(
      port: serverPort.value,
      masterKey: masterKey!,
      configPath: configPath,
    );
  }

  static Future<void> loadFromBackend() async {
    if (!PlatformService().isRemote) return;
    final data = await APIService.instance.getSettings();
    if (data != null) {
      await _applyFromJson(data, fromBackend: true);
    } else {
      log('Failed to load settings from backend', isError: true);
    }
  }

  static Future<void> _saveToBackend() async {
    final jsonMap = await _toJson();
    jsonMap.remove('accounts');
    if (PlatformService().isRemote) {
      jsonMap.remove('server_host');
      jsonMap.remove('server_port');
      jsonMap.remove('require_login');
    }
    final success = await APIService.instance.saveSettings(jsonMap);
    if (!success) {
      log('Failed to save settings to backend', isError: true);
    }
  }

  static void addAccount(Account account) {
    final index = accounts.value.indexWhere((a) {
      if (account.id.isNotEmpty && a.id.isNotEmpty) {
        return a.id == account.id;
      }
      return a.host == account.host && a.port == account.port;
    });
    if (index != -1) {
      final newAccounts = List<Account>.from(accounts.value);
      newAccounts[index] = account;
      accounts.value = newAccounts;
    } else {
      accounts.value = [...accounts.value, account];
    }
  }

  static void removeAccount(Account account) {
    accounts.value = accounts.value
        .where((a) => !(a.host == account.host && a.port == account.port))
        .toList();
  }

  static Future<void> switchAccount(Account account) async {
    isLoggedIn.value = false;
    APIService.instance.clearAuth();
    detachAutoSave();
    serverHost.value = account.host;
    serverPort.value = account.port;

    if (!PlatformService().isRemote) {
      final user = await RinfService.instance.getUserByUsername(
        account.username,
      );
      if (user.found) {
        currentUser.value = User(
          id: user.userId,
          username: user.username,
          displayName: user.displayName,
          role: user.role,
        );
        currentUserId.value = user.userId;
      }

      await saveAll();
      await reloadConfig();
      serverHost.value = account.host;
      serverPort.value = account.port;
      await loadAllUserSettings();

      APIService.isOnline.value = false;
      APIService.serverVersion.value = null;
      SystemService().refreshVersions();
      attachAutoSave();
      APIService.instance.restartPolling();
      return;
    }

    APIService.isOnline.value = false;
    APIService.serverVersion.value = null;
    if (!requireLogin.value && account.apiKey.isNotEmpty) {
      currentUser.value = User(
        id: account.id,
        username: account.username,
        displayName: account.displayName,
        apiKey: account.apiKey,
        role: account.role,
      );
      currentUserId.value = account.id;
      isLoggedIn.value = true;
    }
    await loadFromBackend();
    SystemService().refreshVersions();
    attachAutoSave();
    APIService.instance.restartPolling();
  }
}
