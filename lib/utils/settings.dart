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
  static IOService? _ioServiceInstance;

  /// Platform IO service. Created on first use: the web client attaches
  /// autosave during the login that runs before [init], so a device-only
  /// setting can change before the service exists.
  static IOService get _ioService =>
      _ioServiceInstance ??= IOServiceFactory.create();

  static late String configPath;
  static bool isFirstRun = false;
  static Map<String, dynamic> defaults = {};

  /// Settings that belong to this device only. They are never sent to the
  /// global settings endpoint; in remote mode they are stored in this
  /// device's own config instead. Mirrors the server's device-only list.
  static const Set<String> deviceLocalKeys = {
    'playback_volume',
    'current_user',
    'require_login',
    'check_nightly',
    'accounts',
    'server_host',
    'server_port',
    'theme_mode',
    'use_dynamic_color',
    'custom_color',
    'retreat_to_tray',
  };

  /// Device-only settings the web client keeps in the browser's local
  /// storage. The account list is left out because it carries API keys, the
  /// user id comes from the session cookie, and login / host / port / tray
  /// are fixed on web.
  static const Set<String> webLocalKeys = {
    'playback_volume',
    'check_nightly',
    'theme_mode',
    'use_dynamic_color',
    'custom_color',
  };

  /// Global settings this device loaded from the backend, with the values it
  /// saw. Only globals that changed on this device are pushed back, so a
  /// device never overwrites the server's config with its own stale copy.
  static final Map<String, dynamic> _backendGlobals = {};

  // Your ValueNotifiers
  // A browser has no system tray.
  static final retreatToTray = ValueNotifier<bool>(!kIsWeb);
  static final downloadFolder = ValueNotifier<String>('');
  static final serverHost = ValueNotifier<String>('127.0.0.1');
  static final serverPort = ValueNotifier<int>(8181);
  static final currentUser = ValueNotifier<User?>(null);

  static final accounts = ValueNotifier<List<Account>>([]);
  static final Map<ValueNotifier, VoidCallback> _autoSaveListeners = {};

  /// How long the autosave waits after the last change before it writes.
  static const Duration saveDebounceWindow = Duration(milliseconds: 400);

  /// Changed settings that have been recorded but not written yet. The last
  /// value of a key wins, so a burst of changes on one key is a single write.
  static final Map<String, dynamic> _pendingSaves = {};

  /// The trailing timer that flushes [_pendingSaves].
  static Timer? _saveTimer;

  /// Serializes the flushes, so two writes never race on the same config
  /// file. It also makes `await flushPendingSaves()` mean "everything
  /// recorded so far is on disk".
  static Future<void> _flushChain = Future<void>.value();

  /// Set while the server's settings are applied, so the values the server
  /// just sent are not pushed straight back to it.
  static bool _applyingBackend = false;

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
  static const String defaultNamingPattern =
      '{album_artist??{artist?|/}|/}{album_artist?{album?|/}}{total_discs>1?{disc_padded}|-}{album_artist?{track_padded}| }{multi_artist?{artist}| - }{title}';

  static final namingPattern = ValueNotifier<String>(defaultNamingPattern);

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
      log('Error loading default settings asset: $e', level: LogLevel.error);
    }
  }

  static Future<void> init() async {
    _ioServiceInstance = IOServiceFactory.create();
    await _loadDefaults();

    if (kIsWeb) {
      // The web client keeps its device-only settings in the browser's
      // local storage. Load them before autosave is attached so the stored
      // values are not written straight back.
      await detachAutoSave();
      await _loadLocalSettings();
      await attachAutoSave();
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
          data['server_port'] ?? (defaults['server_port'] ?? 8181);
      await _applyFromJson(data);
    }

    isFirstRun = result.isFirstRun;
    if (isFirstRun) {
      downloadFolder.value = defaultDownloadFolder;
      await saveAll();
    }

    await attachAutoSave();
  }

  /// Loads this device's own settings from the platform's local store.
  static Future<void> _loadLocalSettings() async {
    final data = await _ioService.readLocalSettings();
    // Only the keys that may live in the browser store are read back; a
    // foreign or corrupted entry must not inject settings into this client.
    final local = Map<String, dynamic>.fromEntries(
      data.entries.where((e) => webLocalKeys.contains(e.key)),
    );
    if (local.isEmpty) return;
    await _applyFromJson(local);
  }

  static ThemeMode _themeModeFrom(Object? value, Object? fallback) {
    if (value is int && value >= 0 && value < ThemeMode.values.length) {
      return ThemeMode.values[value];
    }
    if (fallback is int &&
        fallback >= 0 &&
        fallback < ThemeMode.values.length) {
      return ThemeMode.values[fallback];
    }
    return ThemeMode.system;
  }

  static Future<void> _applyFromJson(
    Map<String, dynamic> json, {
    bool fromBackend = false,
  }) async {
    if (!fromBackend) {
      // Device-local settings only ever come from this device's own config.
      // A backend payload must never overwrite them.
      // A browser has no system tray, so the flag is always false there.
      retreatToTray.value = kIsWeb
          ? false
          : (json['retreat_to_tray'] ?? defaults['retreat_to_tray'] ?? true);
      themeMode.value = _themeModeFrom(
        json['theme_mode'],
        defaults['theme_mode'],
      );
      useDynamicColor.value =
          json['use_dynamic_color'] ?? defaults['use_dynamic_color'] ?? true;
      customColor.value =
          json['custom_color'] ?? defaults['custom_color'] ?? 0xFFFF4081;
      checkNightly.value =
          json['check_nightly'] ?? defaults['check_nightly'] ?? false;
      if (json.containsKey('require_login')) {
        requireLogin.value = json['require_login'] ?? false;
      }
      if (json.containsKey('current_user')) {
        currentUserId.value = json['current_user'] as String?;
      }
      if (json['playback_volume'] is num) {
        playbackVolume.value = (json['playback_volume'] as num).toDouble();
      } else if (defaults['playback_volume'] is num) {
        playbackVolume.value = (defaults['playback_volume'] as num).toDouble();
      }

      if (json['accounts'] is List) {
        final accountList = <Account>[];
        for (final accJson in json['accounts']) {
          if (accJson is Map<String, dynamic> && accJson['id'] is String) {
            accountList.add(Account.fromJson(accJson));
          }
        }
        accounts.value = accountList;
      }
    }

    // Global settings are only applied when the payload actually contains
    // them, so a partial response (e.g. the non-admin allowlist) never
    // resets unrelated settings.
    if (json.containsKey('download_folder')) {
      downloadFolder.value = json['download_folder'] ?? '';
    }
    if (json.containsKey('slskd_url')) {
      slskdUrl.value = json['slskd_url'] ?? '';
    }
    if (json.containsKey('nadekodon_url')) {
      nadekodonUrl.value = json['nadekodon_url'] ?? '';
    }
    if (json.containsKey('desired_audio_quality')) {
      desiredAudioQuality.value = json['desired_audio_quality'] ?? 'best';
    }
    if (json.containsKey('default_download_source')) {
      defaultDownloadSource.value = json['default_download_source'] ?? 'slskd';
    }
    if (json.containsKey('naming_pattern')) {
      namingPattern.value =
          json['naming_pattern'] ??
          defaults['naming_pattern'] ??
          defaultNamingPattern;
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
    'naming_pattern': namingPattern.value,
    'accounts': accounts.value.map((e) => e.toJson()).toList(),
  };

  static Future<void> reloadConfig() async {
    // Write the pending changes first, so the reload reflects them.
    await flushPendingSaves();
    final data = await BridgeService.instance.getGlobalSettings();
    if (data != null) {
      _applyingBackend = true;
      try {
        await _applyFromJson(data, fromBackend: true);
      } finally {
        _applyingBackend = false;
      }
    }
  }

  static Future<void> saveAll() async {
    // The full config written below supersedes anything still pending.
    await flushPendingSaves();
    final jsonMap = await _toJson();

    // The device's own config is also the server's config when the app runs
    // the server locally, so everything goes to a single file there.
    if (!kIsWeb && !PlatformService().isRemote) {
      await RinfService.instance.saveLocalConfig(jsonMap);
      return;
    }

    final localPart = <String, dynamic>{};
    final globalPart = <String, dynamic>{};
    for (final entry in jsonMap.entries) {
      if (deviceLocalKeys.contains(entry.key)) {
        localPart[entry.key] = entry.value;
        continue;
      }
      if (!_backendGlobals.containsKey(entry.key)) continue;
      // Only push what changed on this device; the server already has the rest.
      if (_backendGlobals[entry.key] != entry.value) {
        globalPart[entry.key] = entry.value;
      }
    }

    await _saveLocal(localPart);
    await _saveToBackend(globalPart);
  }

  /// Records a changed setting.
  ///
  /// The write is debounced: the change is persisted when no further change
  /// arrives for [saveDebounceWindow]. The notifier itself is already
  /// updated by the caller, so the UI is never delayed by this.
  static void saveChanged(String key, dynamic value) {
    _pendingSaves[key] = value;
    _saveTimer?.cancel();
    _saveTimer = Timer(saveDebounceWindow, () {
      _saveTimer = null;
      unawaited(flushPendingSaves());
    });
  }

  /// Writes every pending change as one batched write.
  ///
  /// Returns a future that completes when the write is done, so a caller
  /// that is about to switch accounts, quit the app or rewrite the whole
  /// config can wait for it.
  static Future<void> flushPendingSaves() {
    _saveTimer?.cancel();
    _saveTimer = null;

    final pending = Map<String, dynamic>.from(_pendingSaves);
    _pendingSaves.clear();
    if (pending.isEmpty) return _flushChain;

    _flushChain = _flushChain.then((_) async {
      try {
        await _writePending(pending);
      } catch (error) {
        log('Failed to flush pending settings: $error', level: LogLevel.error);
      }
    });
    return _flushChain;
  }

  /// Writes one batch of pending changes.
  static Future<void> _writePending(Map<String, dynamic> pending) async {
    // The device's own config is also the server's config when the app runs
    // the server locally, so everything goes to a single file there.
    if (!kIsWeb && !PlatformService().isRemote) {
      await RinfService.instance.saveLocalConfig(pending);
      return;
    }

    final localPart = <String, dynamic>{};
    final globalPart = <String, dynamic>{};
    for (final entry in pending.entries) {
      if (deviceLocalKeys.contains(entry.key)) {
        localPart[entry.key] = entry.value;
        continue;
      }
      globalPart[entry.key] = entry.value;
    }

    await _saveLocal(localPart);

    if (globalPart.isEmpty) return;
    // Global settings are admin-only.
    if (currentUser.value?.role != 'admin') {
      log(
        'Skipped global settings write: the current user is not an admin',
        level: LogLevel.warning,
      );
      return;
    }
    await _saveToBackend(globalPart);
  }

  /// Persists the current user id on this device. Never sent to the server.
  static Future<void> saveCurrentUser(String? userId) async {
    await _saveLocal({'current_user': userId ?? ''});
  }

  static Future<void> attachAutoSave() async {
    await detachAutoSave();

    void add(ValueNotifier n, String key, [dynamic Function()? getValue]) {
      void listener() {
        if (_applyingBackend) return;
        saveChanged(key, getValue != null ? getValue() : n.value);
      }

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
    add(currentUserId, 'current_user');
    add(
      accounts,
      'accounts',
      () => accounts.value.map((e) => e.toJson()).toList(),
    );
  }

  /// Stops the autosave listeners and writes everything still pending.
  static Future<void> detachAutoSave() async {
    // Flush first: a value captured before an account switch would otherwise
    // be written after the switch, with the new account's context.
    await flushPendingSaves();
    _autoSaveListeners.forEach((notifier, listener) {
      notifier.removeListener(listener);
    });
    _autoSaveListeners.clear();
  }

  static Future<void> applyDefaultSettings() async {
    retreatToTray.value = defaults['retreat_to_tray'] ?? true;
    // downloadFolder is usually not reset to default from asset as it's environment dependent
    serverHost.value = defaults['server_host'] ?? '127.0.0.1';
    serverPort.value = defaults['server_port'] ?? 8181;

    if (defaults['theme_mode'] != null) {
      themeMode.value = ThemeMode.values[defaults['theme_mode']];
    }
    useDynamicColor.value = defaults['use_dynamic_color'] ?? true;
    customColor.value = defaults['custom_color'] ?? 0xFFFF4081;
    checkNightly.value = defaults['check_nightly'] ?? false;
    namingPattern.value = defaults['naming_pattern'] ?? defaultNamingPattern;
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

  static Future<String> getCurrentUserApiKey() async {
    final user = currentUser.value;
    if (user == null || user.id.isEmpty) return '';
    if (user.apiKey.isNotEmpty) return user.apiKey;
    if (PlatformService().isRemote) return '';
    try {
      final users = await BridgeService.instance.listUsers();
      for (final u in users) {
        if (u.id != user.id || u.apiKey.isEmpty) continue;
        currentUser.value = User(
          id: user.id,
          username: user.username,
          displayName: user.displayName,
          passwordHash: user.passwordHash,
          apiKey: u.apiKey,
          role: user.role,
          createdAt: user.createdAt,
          updatedAt: user.updatedAt,
        );
        return u.apiKey;
      }
    } catch (e) {
      log('Failed to fetch current user API key: $e', level: LogLevel.error);
    }
    return '';
  }

  static Future<String?> regenerateApiKey(String userId) async {
    if (PlatformService().isRemote) {
      final ok = await APIService.instance.regenerateApiKey();
      return ok ? (currentUser.value?.apiKey ?? '') : null;
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

      final user = currentUser.value;
      if (user != null && result.decryptedApiKey.isNotEmpty) {
        currentUser.value = User(
          id: user.id,
          username: user.username,
          displayName: user.displayName,
          passwordHash: user.passwordHash,
          apiKey: result.decryptedApiKey,
          role: user.role,
          createdAt: user.createdAt,
          updatedAt: user.updatedAt,
        );
      }
      return result.decryptedApiKey;
    }
    return null;
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
    // Write the pending changes first, so the reload reflects them.
    await flushPendingSaves();
    // Drop the snapshot of whichever server we talked to before.
    _backendGlobals.clear();
    final data = await APIService.instance.getSettings();
    if (data != null) {
      _backendGlobals.addAll(
        Map.fromEntries(
          data.entries.where((entry) => !deviceLocalKeys.contains(entry.key)),
        ),
      );
      _applyingBackend = true;
      try {
        await _applyFromJson(data, fromBackend: true);
      } finally {
        _applyingBackend = false;
      }
    } else {
      log('Failed to load settings from backend', level: LogLevel.error);
    }
  }

  /// Sends global settings to the server. Device-local keys are stripped and
  /// never leave this device.
  static Future<void> _saveToBackend(Map<String, dynamic> values) async {
    final payload = Map<String, dynamic>.from(values)
      ..removeWhere((key, _) => deviceLocalKeys.contains(key));
    if (payload.isEmpty) return;
    final success = await APIService.instance.saveSettings(payload);
    if (success) {
      _backendGlobals.addAll(payload);
    } else {
      log('Failed to save settings to backend', level: LogLevel.error);
    }
  }

  /// Persists device-local settings on this device.
  ///
  /// Native keeps them in the device's own config file (through Rinf); the
  /// web client keeps the ones that may live in the browser in its local
  /// storage.
  static Future<void> _saveLocal(Map<String, dynamic> values) async {
    if (values.isEmpty) return;
    if (kIsWeb) {
      // Only the settings that may live in the browser are stored: the
      // account list carries API keys, the user id comes from the session
      // cookie, and login / host / port / tray are fixed on web.
      final storable = Map<String, dynamic>.fromEntries(
        values.entries.where((e) => webLocalKeys.contains(e.key)),
      );
      if (storable.isEmpty) return;
      final saved = await _ioService.writeLocalSettings(storable);
      if (!saved) {
        log(
          'Failed to persist local settings in browser storage',
          level: LogLevel.error,
        );
      }
      return;
    }
    await RinfService.instance.saveLocalConfig(values);
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
    await detachAutoSave();
    isLoggedIn.value = false;
    APIService.instance.clearAuth();
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

      await reloadConfig();
      await saveAll();
      await loadAllUserSettings();

      APIService.isOnline.value = false;
      APIService.serverVersion.value = null;
      SystemService().refreshVersions();
      await attachAutoSave();
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
    await saveCurrentUser(account.id);
    await loadFromBackend();
    SystemService().refreshVersions();
    await attachAutoSave();
    APIService.instance.restartPolling();
  }
}
