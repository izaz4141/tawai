import 'dart:async';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:window_manager/window_manager.dart';

import 'package:rinf/rinf.dart';
import 'package:audio_service/audio_service.dart';
import 'package:tawai/src/bindings/bindings.dart';

import 'package:tawai/services/playback_service.dart';
import 'package:tawai/services/audio_handler.dart';
import 'package:tawai/models/account.dart';
import 'package:tawai/models/user.dart';
import 'package:tawai/ui/app.dart';
import 'package:tawai/utils/log_service.dart';
import 'package:tawai/utils/settings.dart';
import 'package:tawai/utils/io_service.dart';
import 'package:tawai/utils/logger.dart';
import 'package:tawai/utils/system_service.dart';
import 'package:tawai/utils/updater.dart';
import 'package:tawai/utils/api_service.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/rinf_service.dart';
import 'package:tawai/utils/single_instance.dart';
import 'package:tawai/utils/app_lifecycle.dart';
import 'package:tawai/utils/platform_service.dart';
import 'package:tawai/utils/helper.dart';

final _windowListener = _WindowListener();

Future<void> main() async {
  runZonedGuarded(
    () async {
      WidgetsFlutterBinding.ensureInitialized();

      LicenseRegistry.addLicense(() async* {
        final licenseText = await rootBundle.loadString(
          'assets/licenses/AGPLv3-LICENSE',
        );
        yield LicenseEntryWithLineBreaks(['Tawai'], licenseText);
      });

      if (!kIsWeb) {
        await LogService.init();
        await initializeRust(assignRustSignal);
        initRustSignalLogger();
      }

      String? masterKey;
      if (!kIsWeb) {
        await cleanupOldFiles();
        masterKey = await getMasterKey();
        final dbPath = await IOServiceFactory.create().getDatabasePath();
        await BridgeService.instance.initDatabase(
          dbPath,
          masterKey: masterKey!,
        );
      }

      await APIService.instance.init();
      await SettingsManager.init();
      await SystemService().init();

      if (!kIsWeb) {
        BridgeService.instance.startServer(
          port: SettingsManager.serverPort.value,
          masterKey: masterKey!,
          configPath: SettingsManager.configPath,
        );

        final users = await RinfService.instance.listUsers();
        if (users.isNotEmpty && SettingsManager.accounts.value.isEmpty) {
          final accounts = users
              .map(
                (u) => Account(
                  host: '127.0.0.1',
                  port: SettingsManager.serverPort.value,
                  username: u.username,
                  displayName: u.displayName,
                  label: '${u.username}@localhost',
                  role: u.role,
                  apiKey: u.apiKey,
                ),
              )
              .toList();
          SettingsManager.accounts.value = accounts;
          await SettingsManager.saveAll();
        }

        await _resolveCurrentUser();
        await SettingsManager.loadAllUserSettings();
        SettingsManager.currentUserId.addListener(
          () => SettingsManager.loadAllUserSettings(),
        );
        await RinfService.instance.startPeriodicScan();
      }

      if (PlatformService.isDesktop) {
        await SingleInstance.init(() async {
          await PlatformService().focusWindow();
        });
        await PlatformService().initWindow(
          listener: _windowListener,
          onReady: () async {
            await PlatformService().focusWindow();
          },
        );

        if (SettingsManager.retreatToTray.value) {
          await initTray();
        }

        SettingsManager.retreatToTray.addListener(() async {
          if (SettingsManager.retreatToTray.value) {
            await initTray();
          } else {
            await removeTray();
          }
        });
      }

      PlaybackService.instance.init();

      if (kIsWeb || PlatformService.isAndroid) {
        PlaybackService.instance.audioHandler = await AudioService.init(
          builder: () => TawaiAudioHandler(),
          config: const AudioServiceConfig(
            androidNotificationChannelId: 'com.example.tawai.channel.audio',
            androidNotificationChannelName: 'Playback',
            androidNotificationOngoing: true,
            androidStopForegroundOnPause: true,
          ),
        );
      }

      runApp(const Tawai());
    },
    (error, stack) {
      log('Error: $error', isError: true);
      log('Stack: $stack', isError: true);
    },
    zoneSpecification: ZoneSpecification(
      print: (Zone self, ZoneDelegate parent, Zone zone, String line) {
        LogService.recordLog(line);
        parent.print(zone, line);
      },
    ),
  );
}

Future<void> _resolveCurrentUser() async {
  final savedId = SettingsManager.currentUserId.value;
  if (savedId != null) {
    final user = await RinfService.instance.getUserById(savedId);
    if (user.found) {
      SettingsManager.currentUser.value = User(
        id: user.userId,
        username: user.username,
        displayName: user.displayName,
        role: user.role,
      );
      return;
    }
  }
  final accounts = SettingsManager.accounts.value;
  if (accounts.isNotEmpty) {
    final username = accounts.first.username;
    final user = await RinfService.instance.getUserByUsername(username);
    if (user.found) {
      SettingsManager.currentUser.value = User(
        id: user.userId,
        username: user.username,
        displayName: user.displayName,
        role: user.role,
      );
      SettingsManager.currentUserId.value = user.userId;
      await SettingsManager.saveAll();
      return;
    }
  }
}

class _WindowListener extends WindowListener {
  @override
  void onWindowClose() async {
    if (SettingsManager.retreatToTray.value) {
      if (PlatformService.isDesktop) {
        await PlatformService().hideWindow();
      }
    } else {
      await closeApp();
    }
  }
}
