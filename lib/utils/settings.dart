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

/// One notifier bound to a dotted path in the settings document.
///
/// [decode] turns the value stored in the document into the notifier's type,
/// and [encode] turns it back. The defaults are identity for plain values.
class _SettingBinding {
  final String path;
  final ValueNotifier<dynamic> notifier;
  final dynamic Function(dynamic raw) decode;
  final dynamic Function(dynamic value) encode;

  const _SettingBinding(
    this.path,
    this.notifier, {
    this.decode = _identity,
    this.encode = _identity,
  });

  static dynamic _identity(dynamic value) => value;
}

class SettingsManager {
  static const String masterKeyFile = 'master.key';

  /// The settings manifest. It is the single source of truth for the shape,
  /// defaults, scope and capabilities of the global settings document.
  static const String schemaAsset = 'assets/docs/settings_schema.json';

  static IOService? _ioServiceInstance;

  /// Platform IO service. Created on first use: the web client attaches
  /// autosave during the login that runs before [init], so a device-only
  /// setting can change before the service exists.
  static IOService get _ioService =>
      _ioServiceInstance ??= IOServiceFactory.create();

  static late String configPath;
  static bool isFirstRun = false;

  /// The nested default document, folded from the manifest on startup.
  static Map<String, dynamic> defaults = {};

  /// The schema version this build ships with.
  static int schemaVersion = 1;

  /// Set when the document on disk (or on the server) belongs to another
  /// schema version. Global settings stay read-only and are never written;
  /// a remote client may still persist its own device-local settings.
  static bool _readOnly = false;
  static bool get isReadOnly => _readOnly;

  /// Flat manifest entries, keyed by dotted path.
  static Map<String, dynamic> _specs = {};

  /// Every declared path that is device-local, web-local or admin-only.
  static final List<String> _devicePaths = [];
  static final List<String> _webLocalPaths = [];
  static final List<String> _adminPaths = [];
  static final List<String> _secretPaths = [];

  /// Global settings this device loaded from the backend, with the values it
  /// saw. Only globals that changed on this device are pushed back, so a
  /// device never overwrites the server's config with its own stale copy.
  static final Map<String, dynamic> _backendGlobals = {};

  /// True once this device has loaded the server's settings at least once.
  /// Until then a full save must not push local defaults over the server.
  static bool _backendLoaded = false;

  // ValueNotifiers, each bound to a dotted path by [_bindings].
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
  /// value of a path wins, so a burst of changes on one path is one write.
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

  // Default download source
  static final defaultDownloadSource = ValueNotifier<String>('slskd');

  // ListenBrainz Settings (per-user)
  static final listenbrainzToken = ValueNotifier<String>('');

  // Recommendation sources shown in library (comma-separated keys)
  static final includedRecommendations = ValueNotifier<String>('');

  // Playback preferences that survive a queue reset.
  static final playbackLoopMode = ValueNotifier<String>('off');
  static final playbackShuffle = ValueNotifier<bool>(false);

  // Naming Pattern
  static final namingPattern = ValueNotifier<String>('');

  /// The default naming pattern, read from the manifest.
  static String get defaultNamingPattern =>
      (dotted('library.organization.naming_pattern') as String?) ?? '';

  // Per-user metadata preferences
  static final lyricsPrefersync = ValueNotifier<bool>(true);

  // Per-user streaming preferences
  static final preferredBitrate = ValueNotifier<String>('lossless');

  // Per-user playback preferences
  static final replayGainEnabled = ValueNotifier<bool>(false);
  static final replayGainPreamp = ValueNotifier<double>(0.0);

  // Playback preferences
  static final playbackVolume = ValueNotifier<double>(1.0);

  /// Every autosaved setting, in the order the settings UI presents them.
  static final List<_SettingBinding> _bindings = [
    _SettingBinding('interface.window.retreat_to_tray', retreatToTray),
    _SettingBinding(
      'interface.theme.mode',
      themeMode,
      decode: _decodeThemeMode,
      encode: _encodeThemeMode,
    ),
    _SettingBinding('interface.theme.dynamic_color', useDynamicColor),
    _SettingBinding(
      'interface.theme.custom_color',
      customColor,
      decode: _decodeColor,
      encode: _encodeColor,
    ),
    _SettingBinding('playback.volume', playbackVolume, decode: _decodeDouble),
    _SettingBinding('playback.loop_mode', playbackLoopMode),
    _SettingBinding('playback.shuffle', playbackShuffle),
    _SettingBinding('downloads.folder', downloadFolder),
    _SettingBinding('downloads.source', defaultDownloadSource),
    _SettingBinding('downloads.quality', desiredAudioQuality),
    _SettingBinding('library.organization.naming_pattern', namingPattern),
    _SettingBinding('integrations.slskd.url', slskdUrl),
    _SettingBinding('integrations.nadekodon.url', nadekodonUrl),
    _SettingBinding('accounts.current_user', currentUserId),
    _SettingBinding(
      'accounts.saved',
      accounts,
      decode: _decodeAccounts,
      encode: _encodeAccounts,
    ),
    _SettingBinding('server.host', serverHost),
    _SettingBinding('server.port', serverPort, decode: _decodeInt),
    _SettingBinding('server.require_login', requireLogin),
    _SettingBinding('system.check_nightly', checkNightly),
  ];

  // ---------------------------------------------------------------------------
  // Manifest
  // ---------------------------------------------------------------------------

  /// Reads the manifest and folds its defaults into a nested document.
  static Future<void> _loadSchema() async {
    try {
      final raw =
          json.decode(await rootBundle.loadString(schemaAsset))
              as Map<String, dynamic>;
      schemaVersion = (raw['version'] as num?)?.toInt() ?? 1;
      _specs = Map<String, dynamic>.from(raw['settings'] as Map);

      _devicePaths.clear();
      _webLocalPaths.clear();
      _adminPaths.clear();
      _secretPaths.clear();

      defaults = {'schema_version': schemaVersion};
      for (final entry in _specs.entries) {
        final path = entry.key;
        final spec = entry.value;
        if (path.contains('[]')) continue;
        if (spec is Map && spec['scope'] == 'device') _devicePaths.add(path);
        if (spec is Map && spec['web_local'] == true) _webLocalPaths.add(path);
        if (spec is Map && spec['admin'] == true) _adminPaths.add(path);
        if (spec is Map && spec['secret'] == true) _secretPaths.add(path);
        setDotted(defaults, path, spec is Map ? spec['default'] : null);
      }
    } catch (e) {
      log('Error loading settings schema asset: $e', level: LogLevel.error);
    }
  }

  /// Returns the manifest default for a dotted path.
  static dynamic defaultFor(String path) => dotted(path);

  /// Reads a dotted path out of [defaults].
  static dynamic dotted(String path) {
    dynamic current = defaults;
    for (final part in path.split('.')) {
      if (current is! Map || !current.containsKey(part)) return null;
      current = current[part];
    }
    return current;
  }

  /// Writes a value at a dotted path, creating intermediate maps.
  static void setDotted(Map<String, dynamic> root, String path, dynamic value) {
    final parts = path.split('.');
    var current = root;
    for (var i = 0; i < parts.length - 1; i++) {
      final next = current[parts[i]];
      if (next is Map<String, dynamic>) {
        current = next;
      } else {
        final created = <String, dynamic>{};
        current[parts[i]] = created;
        current = created;
      }
    }
    current[parts.last] = value;
  }

  /// Folds a flat map of dotted paths into a nested document.
  static Map<String, dynamic> nest(Map<String, dynamic> flat) {
    final root = <String, dynamic>{};
    flat.forEach((path, value) => setDotted(root, path, value));
    return root;
  }

  /// Looks a dotted path up in [json], distinguishing "missing" from "null".
  static ({bool present, dynamic value}) _lookup(
    Map<String, dynamic> json,
    String path,
  ) {
    dynamic current = json;
    for (final part in path.split('.')) {
      if (current is! Map || !current.containsKey(part)) {
        return (present: false, value: null);
      }
      current = current[part];
    }
    return (present: true, value: current);
  }

  /// True when [path] is [prefix] itself or sits underneath it.
  static bool _isUnder(String path, String prefix) =>
      path == prefix || path.startsWith('$prefix.');

  static bool _matchesAny(String path, List<String> prefixes) =>
      prefixes.any((prefix) => _isUnder(path, prefix));

  static bool isDevicePath(String path) => _matchesAny(path, _devicePaths);
  static bool isWebLocalPath(String path) => _matchesAny(path, _webLocalPaths);
  static bool isAdminPath(String path) => _matchesAny(path, _adminPaths);
  static bool isSecretPath(String path) => _matchesAny(path, _secretPaths);

  /// True when this process runs the server itself, so its device config file
  /// is also the server's config file.
  static bool get _runsLocalServer => !kIsWeb && !PlatformService().isRemote;

  // ---------------------------------------------------------------------------
  // Value conversion
  // ---------------------------------------------------------------------------

  static ThemeMode _decodeThemeMode(dynamic raw) {
    switch (raw) {
      case 'light':
        return ThemeMode.light;
      case 'dark':
        return ThemeMode.dark;
      default:
        return ThemeMode.system;
    }
  }

  static String _encodeThemeMode(dynamic value) {
    return switch (value) {
      ThemeMode.light => 'light',
      ThemeMode.dark => 'dark',
      _ => 'system',
    };
  }

  static double _decodeDouble(dynamic raw) => raw is num ? raw.toDouble() : 1.0;

  static int _decodeInt(dynamic raw) {
    if (raw is num) return raw.toInt();
    final fallback = dotted('server.port');
    return fallback is num ? fallback.toInt() : 8181;
  }

  static int _decodeColor(dynamic raw) {
    if (raw is int) return 0xFF000000 | (raw & 0xFFFFFF);
    if (raw is String && RegExp(r'^#[0-9a-fA-F]{6}$').hasMatch(raw)) {
      return 0xFF000000 | int.parse(raw.substring(1), radix: 16);
    }
    return 0xFFFF4081;
  }

  static String _encodeColor(dynamic value) {
    final argb = value is int ? value : 0xFFFF4081;
    return '#${(argb & 0xFFFFFF).toRadixString(16).padLeft(6, '0').toUpperCase()}';
  }

  static List<Account> _decodeAccounts(dynamic raw) {
    if (raw is! List) return [];
    final result = <Account>[];
    for (final entry in raw) {
      if (entry is Map<String, dynamic> && entry['id'] is String) {
        result.add(Account.fromJson(entry));
      }
    }
    return result;
  }

  static List<dynamic> _encodeAccounts(dynamic value) {
    final list = value is List ? value : const [];
    return [
      for (final account in list)
        if (account is Account) account.toJson(),
    ];
  }

  // ---------------------------------------------------------------------------
  // Loading and applying the document
  // ---------------------------------------------------------------------------

  static Future<void> init() async {
    _ioServiceInstance = IOServiceFactory.create();
    await _loadSchema();

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
    // Only the paths that may live in the browser store are read back; a
    // foreign or corrupted entry must not inject settings into this client.
    final local = Map<String, dynamic>.fromEntries(
      data.entries.where((e) => isWebLocalPath(e.key)),
    );
    if (local.isEmpty) return;
    await _applyFromJson(nest(local));
  }

  static Future<void> _applyFromJson(
    Map<String, dynamic> json, {
    bool fromBackend = false,
  }) async {
    if (fromBackend) {
      final version = json['schema_version'];
      if (version is num && version.toInt() != schemaVersion) {
        if (!_readOnly) {
          log(
            'Refusing settings document with schema version $version; '
            'this build expects $schemaVersion. Settings are read-only.',
            level: LogLevel.error,
          );
        }
        _readOnly = true;
        // A remote client still owns its device-local settings, so keep those
        // bindings; a local server keeps the existing full reset.
        _applyDefaultsToNotifiers(resetDevicePaths: _runsLocalServer);
        return;
      }
    }

    for (final binding in _bindings) {
      if (fromBackend && isDevicePath(binding.path)) continue;
      final found = _lookup(json, binding.path);
      if (!found.present) continue;
      var raw = found.value;
      // A browser has no system tray.
      if (binding.path == 'interface.window.retreat_to_tray' && kIsWeb) {
        raw = false;
      }
      try {
        binding.notifier.value = binding.decode(raw);
      } catch (e) {
        log(
          'Ignoring unusable value for ${binding.path}: $e',
          level: LogLevel.warning,
        );
      }
    }
  }

  static void _applyDefaultsToNotifiers({bool resetDevicePaths = true}) {
    for (final binding in _bindings) {
      if (!resetDevicePaths && isDevicePath(binding.path)) continue;
      var raw = dotted(binding.path);
      if (binding.path == 'interface.window.retreat_to_tray' && kIsWeb) {
        raw = false;
      }
      binding.notifier.value = binding.decode(raw);
    }
  }

  static Map<String, dynamic> _toJsonFlat() => {
    for (final binding in _bindings)
      binding.path: binding.encode(binding.notifier.value),
  };

  /// The current value of every global path this device knows about.
  static Map<String, dynamic> _globalSnapshot() => {
    for (final binding in _bindings)
      if (!isDevicePath(binding.path))
        binding.path: binding.encode(binding.notifier.value),
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
    final localServer = _runsLocalServer;
    if (_readOnly) {
      // A schema mismatch keeps global settings read-only, but a remote client
      // still owns its device-local settings, so persist those and nothing else.
      if (localServer) return;
      await flushPendingSaves();
      final flat = _toJsonFlat();
      await _saveLocalFlat(
        Map.fromEntries(flat.entries.where((e) => isDevicePath(e.key))),
      );
      return;
    }
    // The full config written below supersedes anything still pending.
    await flushPendingSaves();
    final flat = _toJsonFlat();

    // The device's own config is also the server's config when the app runs
    // the server locally, so everything goes to a single file there.
    if (localServer) {
      await RinfService.instance.saveLocalConfig(nest(flat));
      return;
    }

    await _saveLocalFlat(
      Map.fromEntries(flat.entries.where((e) => isDevicePath(e.key))),
    );

    final globals = Map<String, dynamic>.fromEntries(
      flat.entries.where((e) => !isDevicePath(e.key)),
    );
    if (!_backendLoaded) return;
    final changed = <String, dynamic>{};
    for (final entry in globals.entries) {
      if (jsonEncode(_backendGlobals[entry.key]) != jsonEncode(entry.value)) {
        changed[entry.key] = entry.value;
      }
    }
    if (changed.isEmpty) return;
    // Global settings are admin-only.
    if (currentUser.value?.role != 'admin') {
      log(
        'Skipped global settings write: the current user is not an admin',
        level: LogLevel.warning,
      );
      return;
    }
    await _saveToBackendFlat(changed);
  }

  /// Records a changed setting by dotted path.
  ///
  /// The write is debounced: the change is persisted when no further change
  /// arrives for [saveDebounceWindow]. The notifier itself is already
  /// updated by the caller, so the UI is never delayed by this.
  static void saveChanged(String path, dynamic value) {
    _pendingSaves[path] = value;
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
    final localServer = _runsLocalServer;
    // A schema mismatch makes a local server fully read-only. A remote client
    // may still write its device-local paths below.
    if (_readOnly && localServer) return;

    // The device's own config is also the server's config when the app runs
    // the server locally, so everything goes to a single file there.
    if (localServer) {
      await RinfService.instance.saveLocalConfig(nest(pending));
      return;
    }

    final localFlat = <String, dynamic>{};
    final globalFlat = <String, dynamic>{};
    for (final entry in pending.entries) {
      if (isDevicePath(entry.key)) {
        localFlat[entry.key] = entry.value;
      } else {
        globalFlat[entry.key] = entry.value;
      }
    }

    await _saveLocalFlat(localFlat);

    // Global settings must never be written while the schema mismatch keeps
    // them read-only.
    if (_readOnly) return;

    if (globalFlat.isEmpty) return;
    // Global settings are admin-only.
    if (currentUser.value?.role != 'admin') {
      log(
        'Skipped global settings write: the current user is not an admin',
        level: LogLevel.warning,
      );
      return;
    }
    await _saveToBackendFlat(globalFlat);
  }

  /// Persists the current user id on this device. Never sent to the server.
  static Future<void> saveCurrentUser(String? userId) async {
    await _saveLocalFlat({'accounts.current_user': userId ?? ''});
  }

  static Future<void> attachAutoSave() async {
    await detachAutoSave();

    for (final binding in _bindings) {
      void listener() {
        if (_applyingBackend) return;
        saveChanged(binding.path, binding.encode(binding.notifier.value));
      }

      binding.notifier.addListener(listener);
      _autoSaveListeners[binding.notifier] = listener;
    }
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
    // downloadFolder is usually not reset to default from the manifest as it
    // is environment dependent.
    for (final binding in _bindings) {
      if (binding.path == 'downloads.folder') continue;
      var raw = dotted(binding.path);
      if (binding.path == 'interface.window.retreat_to_tray' && kIsWeb) {
        raw = false;
      }
      binding.notifier.value = binding.decode(raw);
    }
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
    final payload = nest({'integrations.slskd.api_key': plainKey});
    if (PlatformService().isRemote) {
      await APIService.instance.saveSettings(payload);
      return;
    }
    await RinfService.instance.saveConfig(payload);
  }

  static Future<void> saveNadekodonApiKey(String plainKey) async {
    nadekodonApiKey.value = plainKey;
    if (plainKey.isEmpty) return;
    final payload = nest({'integrations.nadekodon.api_key': plainKey});
    if (PlatformService().isRemote) {
      await APIService.instance.saveSettings(payload);
      return;
    }
    await RinfService.instance.saveConfig(payload);
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
    _backendLoaded = false;
    _readOnly = false;
    final data = await APIService.instance.getSettings();
    if (data != null) {
      _applyingBackend = true;
      try {
        await _applyFromJson(data, fromBackend: true);
      } finally {
        _applyingBackend = false;
      }
      if (!_readOnly) {
        _backendGlobals.addAll(_globalSnapshot());
        _backendLoaded = true;
      }
    } else {
      log('Failed to load settings from backend', level: LogLevel.error);
    }
  }

  /// Sends global settings to the server. Device-local paths are stripped and
  /// never leave this device.
  static Future<void> _saveToBackendFlat(Map<String, dynamic> values) async {
    final payload = Map<String, dynamic>.from(values)
      ..removeWhere((path, _) => isDevicePath(path));
    if (payload.isEmpty) return;
    payload['schema_version'] = schemaVersion;
    final success = await APIService.instance.saveSettings(nest(payload));
    if (success) {
      _backendGlobals.addAll(values);
    } else {
      log('Failed to save settings to backend', level: LogLevel.error);
    }
  }

  /// Persists device-local settings on this device.
  ///
  /// Native keeps them in the device's own config file (through Rinf); the
  /// web client keeps the ones that may live in the browser in its local
  /// storage.
  static Future<void> _saveLocalFlat(Map<String, dynamic> values) async {
    if (values.isEmpty) return;
    if (kIsWeb) {
      // Only the settings that may live in the browser are stored: the
      // account list carries API keys, the user id comes from the session
      // cookie, and login / host / port / tray are fixed on web.
      final storable = Map<String, dynamic>.fromEntries(
        values.entries.where((e) => isWebLocalPath(e.key)),
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
    await RinfService.instance.saveLocalConfig(nest(values));
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
