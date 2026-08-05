import 'dart:async';
import 'package:flutter/foundation.dart';

import 'package:tawai/src/bindings/bindings.dart';

import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/settings.dart';

class ScanService {
  static final ScanService _instance = ScanService._();
  static ScanService get instance => _instance;
  ScanService._();

  final ValueNotifier<bool> isScanning = ValueNotifier(false);
  final ValueNotifier<ScanProgressSignal?> progress = ValueNotifier(null);

  static const Duration _idleInterval = Duration(seconds: 5);
  static const Duration _activeInterval = Duration(seconds: 1);

  int _refCount = 0;
  Timer? _statusPollTimer;
  bool _polling = false;

  String get _userId => SettingsManager.currentUser.value?.id ?? '';

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
  // Polling lifecycle (refCount-gated by UI consumers)
  // ---------------------------------------------------------------------------

  void acquire() {
    _refCount++;
    if (!_polling) {
      _polling = true;
      _scheduleNextPoll();
    }
  }

  void release() {
    _refCount--;
    if (_refCount <= 0) {
      _refCount = 0;
      _stopStatusPolling();
    }
  }

  void _startStatusPolling() {
    if (_polling) return;
    _polling = true;
    _scheduleNextPoll();
  }

  void _stopStatusPolling() {
    _statusPollTimer?.cancel();
    _statusPollTimer = null;
    _polling = false;
  }

  void _scheduleNextPoll() {
    _statusPollTimer?.cancel();
    final interval = isScanning.value ? _activeInterval : _idleInterval;
    _statusPollTimer = Timer(interval, _pollScanStatus);
  }

  // ---------------------------------------------------------------------------
  // Scan
  // ---------------------------------------------------------------------------

  Future<void> incrementalScan() async {
    if (isScanning.value) return;
    await _performScan(force: false);
  }

  Future<void> forceRescan() async {
    if (isScanning.value) return;
    await _performScan(force: true);
  }

  Future<void> _performScan({required bool force}) async {
    _resetProgress();
    isScanning.value = true;
    final result = await BridgeService.instance.scanLibrary(
      userId: _userId,
      force: force,
    );
    if (result.started) {
      if (!_polling) _startStatusPolling();
    } else {
      debugPrint('Scan failed to start: ${result.error}');
      isScanning.value = false;
      progress.value = null;
    }
  }

  void _resetProgress() {
    progress.value = null;
  }

  void _finalizeScan() {
    isScanning.value = false;
    progress.value = null;
  }

  // ---------------------------------------------------------------------------
  // Status polling (adaptive: slow when idle, fast while scanning)
  // ---------------------------------------------------------------------------

  Future<void> _pollScanStatus() async {
    final result = await BridgeService.instance.getScanStatus();
    if (result.running) {
      if (!isScanning.value) isScanning.value = true;
      if (result.progress != null) {
        progress.value = result.progress;
      }
    } else if (!result.running && isScanning.value) {
      _finalizeScan();
    }
    if (_polling) {
      _scheduleNextPoll();
    }
  }

  void dispose() {
    _stopStatusPolling();
  }
}
