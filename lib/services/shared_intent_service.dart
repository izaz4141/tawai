import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter_sharing_intent/flutter_sharing_intent.dart';
import 'package:flutter_sharing_intent/model/sharing_file.dart';

import 'package:tawai/services/playback_service.dart';
import 'package:tawai/utils/io_service.dart';
import 'package:tawai/utils/logger.dart';

/// Receives audio files shared from other Android apps via the system share
/// sheet or "Open with" and hands them to the player.
class SharedIntentService {
  static final SharedIntentService _instance = SharedIntentService._();
  static SharedIntentService get instance => _instance;
  SharedIntentService._();

  static const MethodChannel _contentChannel = MethodChannel(
    'tawai/shared_file',
  );

  static const Set<String> _audioExtensions = {
    '.mp3',
    '.aac',
    '.m4a',
    '.m4b',
    '.flac',
    '.wav',
    '.ogg',
    '.oga',
    '.opus',
    '.wma',
    '.aiff',
    '.aif',
    '.amr',
    '.mp4',
    '.alac',
  };

  StreamSubscription<List<SharedFile>>? _sub;
  final Set<String> _recent = {};
  Timer? _dedupeTimer;

  void init() {
    if (_sub != null) return;
    _sub = FlutterSharingIntent.instance.getMediaStream().listen(
      _handleFiles,
      onError: _handleError,
    );

    FlutterSharingIntent.instance
        .getInitialSharing()
        .then(_handleFiles)
        .catchError(
          (Object e) => log('getInitialSharing error: $e', isError: true),
        );
    FlutterSharingIntent.instance.reset();
  }

  Future<void> _handleFiles(List<SharedFile> files) async {
    final audio = files.where(_isAudio).toList();
    if (audio.isEmpty) return;

    for (final file in audio) {
      final value = file.value;
      if (value == null || value.isEmpty) continue;
      if (!_dedupe(value)) continue;
      final path = await _materialize(value);
      if (path == null) continue;
      await PlaybackService.instance.playExternalFile(path);
    }
  }

  bool _isAudio(SharedFile file) {
    final mimeType = file.mimeType ?? '';
    if (mimeType.toLowerCase().startsWith('audio')) return true;
    return _audioExtensions.contains(_extensionOf(file.value ?? ''));
  }

  bool _dedupe(String value) {
    if (_recent.contains(value)) return false;
    _recent.add(value);
    _dedupeTimer?.cancel();
    _dedupeTimer = Timer(const Duration(seconds: 30), _recent.clear);
    return true;
  }

  String _extensionOf(String value) {
    final lower = value.toLowerCase();
    final index = lower.lastIndexOf('.');
    if (index < 0) return '';
    return lower.substring(index);
  }

  /// Resolves the shared value to a stable local file path. Values coming from
  /// the share sheet are already local paths (the plugin copies them); `content://`
  /// URIs (from "Open with") are copied via the native channel.
  Future<String?> _materialize(String value) async {
    try {
      if (value.startsWith('content://')) {
        return await _contentChannel.invokeMethod<String>('copyContentUri', {
          'uri': value,
        });
      }
      final uri = Uri.tryParse(value);
      if (uri != null && uri.scheme == 'file') {
        return _copyToTemp(File(uri.toFilePath()));
      }
      final src = File(value);
      if (!await src.exists()) return null;
      return await _copyToTemp(src);
    } catch (e) {
      log('Shared file materialize error: $e', isError: true);
      return null;
    }
  }

  Future<String> _copyToTemp(File src) async {
    final io = IOServiceFactory.create();
    final dir = await io.getTempDir();
    final destDir = '$dir/tawai_shared';
    await io.createDirectory(destDir, recursive: true);
    final dest = File('$destDir/${src.uri.pathSegments.last}');
    if (await dest.exists()) {
      await dest.delete();
    }
    await src.copy(dest.path);
    return dest.path;
  }

  void _handleError(Object e) {
    log('Shared intent stream error: $e', isError: true);
  }

  void dispose() {
    _sub?.cancel();
    _sub = null;
    _dedupeTimer?.cancel();
  }
}
