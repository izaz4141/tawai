import 'dart:async';
import 'package:flutter/foundation.dart';

import 'package:tawai/src/bindings/bindings.dart';
import 'package:rinf/rinf.dart';

import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/platform_service.dart';
import 'package:tawai/utils/settings.dart';

class ScanService {
  static final ScanService _instance = ScanService._();
  static ScanService get instance => _instance;
  ScanService._() {
    _initAmbientListeners();
    _startStatusPolling();
  }

  final ValueNotifier<bool> isScanning = ValueNotifier(false);
  final ValueNotifier<ScanProgressSignal?> progress = ValueNotifier(null);

  // Per-operation subscription (both local and remote user-initiated scans)
  StreamSubscription<ScanProgressSignal>? _scanSub;

  // Ambient listener for local periodic scans (hub-initiated)
  StreamSubscription<RustSignalPack<ScanProgressSignal>>? _ambientProgressSub;

  Timer? _statusPollTimer;

  String get _userId => SettingsManager.currentUser.value?.id ?? '';
  bool get _isRemote => PlatformService().isRemote;

  void _initAmbientListeners() {
    _ambientProgressSub = ScanProgressSignal.rustSignalStream.listen((pack) {
      _processProgress(pack.message);
      if (pack.message.complete) {
        _finalizeScan();
      }
    });
  }

  // ---------------------------------------------------------------------------
  // Library Sources
  // ---------------------------------------------------------------------------

  Future<List<LibrarySourceInfo>> getLibrarySources() async {
    return BridgeService.instance.listLibrarySources(_userId);
  }

  Future<bool> addSource(
    String url,
    String name, {
    String sourceType = 'local',
  }) async {
    final result = await BridgeService.instance.addLibrarySource(
      _userId,
      url,
      name,
      sourceType,
    );
    return result.success;
  }

  Future<bool> removeSource(String sourceId) async {
    return BridgeService.instance.removeLibrarySource(_userId, sourceId);
  }

  // ---------------------------------------------------------------------------
  // Scan
  // ---------------------------------------------------------------------------

  void incrementalScan() {
    if (isScanning.value) return;
    _resetProgress();
    isScanning.value = true;
    _performScan(force: false);
  }

  void forceRescan() {
    if (isScanning.value) return;
    _resetProgress();
    isScanning.value = true;
    _performScan(force: true);
  }

  void _resetProgress() {
    progress.value = null;
  }

  void _performScan({required bool force}) {
    _scanSub?.cancel();
    _scanSub = BridgeService.instance
        .scanLibrary(userId: _userId, force: force)
        .listen(
          _processProgress,
          onDone: _finalizeScan,
          onError: (_) => _finalizeScan(),
        );
  }

  void _processProgress(ScanProgressSignal signal) {
    if (!isScanning.value && signal.stage.isNotEmpty) {
      isScanning.value = true;
    }
    progress.value = signal;
  }

  void _finalizeScan() {
    isScanning.value = false;
    progress.value = null;
    _scanSub?.cancel();
    _scanSub = null;
    _stopStatusPolling();
    _startStatusPolling();
  }

  // ---------------------------------------------------------------------------
  // Remote status polling (detect scans by other users)
  // ---------------------------------------------------------------------------

  void _startStatusPolling() {
    if (!_isRemote) return;
    _statusPollTimer?.cancel();
    _statusPollTimer = Timer.periodic(const Duration(seconds: 5), (_) {
      _pollScanStatus();
    });
  }

  void _stopStatusPolling() {
    _statusPollTimer?.cancel();
    _statusPollTimer = null;
  }

  Future<void> _pollScanStatus() async {
    final result = await BridgeService.instance.getScanStatus();
    if (result.running) {
      if (!isScanning.value) isScanning.value = true;
      if (result.progress != null) {
        progress.value = ScanProgressSignal(
          id: '',
          currentFile: result.progress!['current_file'] as String? ?? '',
          filesScanned:
              (result.progress!['files_scanned'] as num?)?.toInt() ?? 0,
          totalFiles: (result.progress!['total_files'] as num?)?.toInt() ?? 0,
          stage: result.progress!['stage'] as String? ?? '',
          complete: result.progress!['complete'] as bool? ?? false,
          tracksFound: (result.progress!['tracks_found'] as num?)?.toInt() ?? 0,
          newTracks: (result.progress!['new_tracks'] as num?)?.toInt() ?? 0,
          duplicates: (result.progress!['duplicates'] as num?)?.toInt() ?? 0,
          deleted: (result.progress!['deleted'] as num?)?.toInt() ?? 0,
          currentSource: result.progress!['current_source'] as String? ?? '',
          error: result.progress!['error'] as String?,
        );
      }
    } else if (!result.running && isScanning.value) {
      _finalizeScan();
    }
  }

  void dispose() {
    _stopStatusPolling();
    _scanSub?.cancel();
    _ambientProgressSub?.cancel();
  }
}
