import 'dart:async';
import 'dart:math' as math;
import 'package:flutter/foundation.dart';
import 'package:just_audio/just_audio.dart';
import 'package:just_audio_media_kit/just_audio_media_kit.dart';
import 'package:synchronized/synchronized.dart';

import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/settings.dart' show SettingsManager;
import 'package:tawai/utils/helper.dart';
import 'package:tawai/utils/logger.dart';

class QueueItem {
  final TrackInfo track;
  final SourceCache? source;

  const QueueItem({required this.track, this.source});

  QueueItem copyWith({TrackInfo? track, SourceCache? source}) {
    return QueueItem(track: track ?? this.track, source: source ?? this.source);
  }
}

class QueueState {
  final List<QueueItem> items;
  final int currentIndex;

  const QueueState({this.items = const [], this.currentIndex = 0});

  QueueItem? get currentItem => items.isNotEmpty ? items[currentIndex] : null;

  TrackInfo? get currentTrack => currentItem?.track;
}

class PlaybackService {
  static final PlaybackService _instance = PlaybackService._();
  static PlaybackService get instance => _instance;
  PlaybackService._();

  final AudioPlayer _player = AudioPlayer();

  final playerState = ValueNotifier<PlayerState>(
    PlayerState(false, ProcessingState.idle),
  );
  final position = ValueNotifier<Duration>(Duration.zero);
  final duration = ValueNotifier<Duration>(Duration.zero);
  final volume = ValueNotifier<double>(1.0);
  final queue = ValueNotifier<QueueState>(QueueState());
  final shuffleModeEnabled = ValueNotifier<bool>(false);
  final loopMode = ValueNotifier<LoopMode>(LoopMode.off);

  TrackInfo? get currentTrack => queue.value.currentTrack;

  final Lock _actionLock = Lock();
  final Map<String, Future<SourceCache?>> _inflight = {};
  static const int _prefetchAhead = 2;
  static const int _prefetchBehind = 2;

  String? _loadedTrackId;
  DateTime? _trackStartTime;
  List<QueueItem> _orderedItems = const [];
  _PreviewRestore? _previewRestore;
  int _sourceGeneration = 0;

  late StreamSubscription<PlayerState> _playerStateSub;
  late StreamSubscription<Duration> _positionSub;
  late StreamSubscription<Duration?> _durationSub;

  double _userVolume = 1.0;
  late VoidCallback _replayGainEnabledSub;
  late VoidCallback _replayGainPreampSub;

  void init() {
    JustAudioMediaKit.ensureInitialized();

    _playerStateSub = _player.playerStateStream.listen((state) {
      playerState.value = state;
      if (state.processingState == ProcessingState.completed) {
        final gen = _sourceGeneration;
        unawaited(_handleCompleted(gen));
      }
    });
    _positionSub = _player.positionStream.listen((pos) {
      position.value = pos;
    });
    _durationSub = _player.durationStream.listen((dur) {
      duration.value = dur ?? Duration.zero;
    });
    _userVolume = SettingsManager.playbackVolume.value;
    volume.value = _userVolume;
    _player.setVolume(_userVolume);
    _replayGainEnabledSub = () {
      unawaited(_applyEffectiveVolume());
    };
    SettingsManager.replayGainEnabled.addListener(_replayGainEnabledSub);
    _replayGainPreampSub = () {
      unawaited(_applyEffectiveVolume());
    };
    SettingsManager.replayGainPreamp.addListener(_replayGainPreampSub);
  }

  void dispose() {
    _playerStateSub.cancel();
    _positionSub.cancel();
    _durationSub.cancel();
    SettingsManager.replayGainEnabled.removeListener(_replayGainEnabledSub);
    SettingsManager.replayGainPreamp.removeListener(_replayGainPreampSub);
    playerState.dispose();
    position.dispose();
    duration.dispose();
    volume.dispose();
    queue.dispose();
    shuffleModeEnabled.dispose();
    loopMode.dispose();
    _player.dispose();
  }

  Future<bool> play(List<TrackInfo> tracks, {int startIndex = 0}) async {
    if (tracks.isEmpty) return false;
    return _actionLock.synchronized(() async {
      await _reset();
      playerState.value = PlayerState(false, ProcessingState.loading);
      final idx = startIndex.clamp(0, tracks.length - 1).toInt();
      _orderedItems = [for (final t in tracks) QueueItem(track: t)];
      queue.value = QueueState(items: _orderedItems, currentIndex: idx);
      final ok = await _loadTrackAt(idx);
      if (!ok) {
        playerState.value = PlayerState(false, ProcessingState.idle);
        return false;
      }
      return true;
    });
  }

  Future<void> preview(DiscoveryRecording rec) async {
    await _actionLock.synchronized(() async {
      final q = queue.value;
      _previewRestore ??= _PreviewRestore(
        items: List<QueueItem>.of(q.items),
        index: q.currentIndex,
        position: position.value,
        playing: playerState.value.playing,
      );

      final syntheticTrack = TrackInfo(
        id: rec.id,
        title: rec.title,
        albumId: '',
        albumTitle: rec.albumTitle ?? '',
        artistsString: rec.artist,
        artists: [
          ArtistInfo(
            id: rec.artistId ?? rec.artist,
            name: rec.artist,
            sortName: null,
            mbid: rec.artistId,
            thumbnailUrl: null,
            albumCount: 0,
            trackCount: 0,
          ),
        ],
        trackNum: null,
        discNum: null,
        durationSecs: rec.durationSecs,
        filePath: '',
        fileSize: null,
        bitrate: null,
        mbidRecording: rec.id,
        artistMbid: rec.artistId,
        albumMbid: null,
        lyrics: null,
        releaseDate: null,
        source: 'preview',
        sourceType: 'preview',
        genres: [],
      );

      playerState.value = PlayerState(false, ProcessingState.loading);
      await _player.stop();
      queue.value = QueueState(
        items: [QueueItem(track: syntheticTrack)],
        currentIndex: 0,
      );

      final result = await BridgeService.instance.previewTrack(syntheticTrack);
      if (result.error != null || result.url == null) {
        log('Preview error: ${result.error}', isError: true);
        await _restoreAfterPreview();
        return;
      }

      final item = QueueItem(
        track: syntheticTrack,
        source: SourceCache(result.url!, null),
      );
      queue.value = QueueState(items: [item], currentIndex: 0);
      _loadedTrackId = rec.id;
      _sourceGeneration++;
      await _player.setAudioSource(_buildSource(item.track, item.source!));
      await _applyEffectiveVolume();
      _beginTrack(syntheticTrack);
      await _player.play();
    });
  }

  Future<void> playNext() async {
    await _actionLock.synchronized(() async {
      final q = queue.value;
      if (q.items.isEmpty) return;
      _scrobbleIfNeeded(q.currentTrack);
      var next = q.currentIndex + 1;
      if (next >= q.items.length) {
        if (loopMode.value == LoopMode.all) {
          next = 0;
        } else {
          await _finish();
          return;
        }
      }
      await _advanceTo(next);
    });
  }

  Future<void> playPrevious() async {
    await _actionLock.synchronized(() async {
      final q = queue.value;
      if (q.items.isEmpty) return;
      if (position.value > const Duration(seconds: 15) || q.currentIndex == 0) {
        await seek(Duration.zero);
        return;
      }
      _scrobbleIfNeeded(q.currentTrack);
      await _advanceTo(q.currentIndex - 1);
    });
  }

  Future<void> playTrackAt(int index) async {
    await _actionLock.synchronized(() async {
      final q = queue.value;
      if (q.items.isEmpty || index < 0 || index >= q.items.length) return;
      _scrobbleIfNeeded(q.currentTrack);
      queue.value = QueueState(items: q.items, currentIndex: index);
      await _loadTrackAt(index);
    });
  }

  Future<void> queueNext(TrackInfo track) async {
    final q = queue.value;
    if (q.items.isEmpty) return;
    final insertAt = q.currentIndex + 1;
    final newItems = List<QueueItem>.of(q.items)
      ..insert(insertAt, QueueItem(track: track));
    queue.value = QueueState(items: newItems, currentIndex: q.currentIndex);

    final curId = q.currentItem?.track.id;
    final orderIdx = curId == null
        ? -1
        : _orderedItems.indexWhere((i) => i.track.id == curId);
    final newOrdered = List<QueueItem>.of(_orderedItems)
      ..insert(orderIdx == -1 ? 0 : orderIdx + 1, QueueItem(track: track));
    _orderedItems = newOrdered;

    unawaited(_prefetchWindow());
  }

  Future<void> queueLast(TrackInfo track) async {
    final q = queue.value;
    final newItems = List<QueueItem>.of(q.items)..add(QueueItem(track: track));
    queue.value = QueueState(items: newItems, currentIndex: q.currentIndex);
    _orderedItems = [..._orderedItems, QueueItem(track: track)];
    unawaited(_prefetchWindow());
  }

  Future<void> removeFromQueue(int index) async {
    await _actionLock.synchronized(() async {
      final q = queue.value;
      if (index < 0 || index >= q.items.length) return;
      final removedId = q.items[index].track.id;
      _inflight.remove(removedId);
      _orderedItems = [
        for (final i in _orderedItems)
          if (i.track.id != removedId) i,
      ];
      final newItems = List<QueueItem>.of(q.items)..removeAt(index);
      if (newItems.isEmpty) {
        await _reset();
        return;
      }
      final newIdx = index < q.currentIndex
          ? q.currentIndex - 1
          : index == q.currentIndex
          ? q.currentIndex.clamp(0, newItems.length - 1).toInt()
          : q.currentIndex;
      queue.value = QueueState(items: newItems, currentIndex: newIdx);
      if (index == q.currentIndex && _loadedTrackId == removedId) {
        await _advanceTo(newIdx);
      } else {
        unawaited(_prefetchWindow());
      }
    });
  }

  Future<void> moveQueueItem(int from, int to) async {
    final q = queue.value;
    if (from < 0 || from >= q.items.length || to < 0 || to >= q.items.length) {
      return;
    }
    final idsBefore = [for (final i in q.items) i.track.id];
    final newItems = List<QueueItem>.of(q.items);
    final item = newItems.removeAt(from);
    newItems.insert(to, item);
    final idsAfter = [for (final i in newItems) i.track.id];
    _orderedItems = [
      for (final id in idsAfter) _orderedItems[idsBefore.indexOf(id)],
    ];
    var newIdx = q.currentIndex;
    if (from == q.currentIndex) {
      newIdx = to;
    } else if (from < q.currentIndex && to >= q.currentIndex) {
      newIdx = q.currentIndex - 1;
    } else if (from > q.currentIndex && to <= q.currentIndex) {
      newIdx = q.currentIndex + 1;
    }
    queue.value = QueueState(items: newItems, currentIndex: newIdx);
  }

  Future<void> toggleShuffle({bool? force}) async {
    final enable = force ?? !shuffleModeEnabled.value;
    if (enable == shuffleModeEnabled.value) return;
    final q = queue.value;
    final currentId = q.currentItem?.track.id;
    if (enable) {
      _orderedItems = List<QueueItem>.of(q.items);
      final shuffled = List<QueueItem>.of(q.items)..shuffle();
      final idx = currentId != null
          ? shuffled.indexWhere((i) => i.track.id == currentId)
          : 0;
      queue.value = QueueState(
        items: shuffled,
        currentIndex: idx >= 0 ? idx : 0,
      );
    } else {
      final restored = _orderedItems;
      final idx = currentId != null
          ? restored.indexWhere((i) => i.track.id == currentId)
          : 0;
      queue.value = QueueState(
        items: List<QueueItem>.of(restored),
        currentIndex: idx >= 0 ? idx : 0,
      );
    }
    shuffleModeEnabled.value = enable;
    unawaited(_prefetchWindow());
  }

  void cycleLoopMode() {
    const modes = [LoopMode.off, LoopMode.all, LoopMode.one];
    final next = (modes.indexOf(loopMode.value) + 1) % modes.length;
    loopMode.value = modes[next];
  }

  Future<void> clearQueue() async {
    await _actionLock.synchronized(() => _reset());
  }

  Future<void> togglePlayPause() async {
    if (playerState.value.playing) {
      await _player.pause();
    } else {
      await _player.play();
    }
  }

  Future<void> seek(Duration pos) async {
    await _player.seek(pos);
  }

  Future<void> setVolume(double vol) async {
    final clamped = vol.clamp(0.0, 1.0).toDouble();
    _userVolume = clamped;
    await _applyEffectiveVolume();
    volume.value = clamped;
    SettingsManager.playbackVolume.value = clamped;
  }

  /// Effective ReplayGain gain in dB for a track, including preamp, capped so
  /// the track's peak cannot be driven past full scale (clipping).
  /// Returns 0.0 when disabled or the track has no gain value.
  double _gainDb(TrackInfo? track) {
    if (!SettingsManager.replayGainEnabled.value) return 0.0;
    final g = track?.trackGain;
    if (g == null) return 0.0;
    final preamp = SettingsManager.replayGainPreamp.value;
    final desired = (g + preamp).clamp(-15.0, 15.0);
    final peak = track?.trackPeak;
    if (peak == null || peak <= 0.0) return desired;
    final maxSafe = -20 * math.log(math.min(peak, 1.0)) / math.ln10;
    return math.min(desired, maxSafe);
  }

  /// Linear volume factor derived from the current track's gain.
  double get _gainFactor =>
      math.pow(10, _gainDb(currentTrack) / 20).toDouble();

  /// Apply the user volume multiplied by the ReplayGain factor.
  Future<void> _applyEffectiveVolume() async {
    await _player.setVolume(_userVolume * _gainFactor);
  }

  Future<TrackInfo?> fetchTrackInfo(String trackId) async {
    return BridgeService.instance.getTrackInfo(trackId);
  }

  // ---------------------------------------------------------------------------
  // Engine
  // ---------------------------------------------------------------------------

  Future<void> _handleCompleted(int emittedGeneration) async {
    await _actionLock.synchronized(() async {
      if (emittedGeneration != _sourceGeneration) return;
      if (_previewRestore != null) {
        await _restoreAfterPreview();
        return;
      }
      final q = queue.value;
      if (q.items.isEmpty) return;
      if (_loadedTrackId != q.currentTrack?.id) return;

      _scrobbleIfNeeded(q.currentTrack);

      switch (loopMode.value) {
        case LoopMode.one:
          _beginTrack(q.currentTrack!);
          await _player.seek(Duration.zero);
          await _player.play();
          return;
        case LoopMode.all:
          final next = (q.currentIndex + 1) % q.items.length;
          await _advanceTo(next);
          return;
        case LoopMode.off:
          final next = q.currentIndex + 1;
          if (next >= q.items.length) {
            await _finish();
          } else {
            await _advanceTo(next);
          }
          return;
      }
    });
  }

  Future<void> _advanceTo(int index) async {
    if (queue.value.items.isEmpty) {
      await _finish();
      return;
    }
    for (var i = index; i < queue.value.items.length; i++) {
      queue.value = QueueState(items: queue.value.items, currentIndex: i);
      if (await _loadTrackAt(i)) return;
    }
    await _finish();
  }

  Future<bool> _loadTrackAt(int index, {bool autoPlay = true}) async {
    final items = queue.value.items;
    if (index < 0 || index >= items.length) return false;

    var item = items[index];
    if (item.source == null) {
      final cache = await _resolveSource(item.track);
      if (cache == null) return false;
      item = item.copyWith(source: cache);
      _attachSource(item.track.id, cache);
    }

    playerState.value = PlayerState(false, ProcessingState.loading);
    await _player.setAudioSource(_buildSource(item.track, item.source!));
    await _applyEffectiveVolume();
    _loadedTrackId = item.track.id;
    _sourceGeneration++;
    _beginTrack(item.track);
    if (autoPlay) {
      await _player.play();
    }
    unawaited(_prefetchWindow());
    return true;
  }

  Future<SourceCache?> _resolveSource(TrackInfo track) {
    final inflight = _inflight[track.id];
    if (inflight != null) return inflight;
    final future = _fetchSource(track);
    _inflight[track.id] = future;
    return future;
  }

  Future<SourceCache?> _fetchSource(TrackInfo track) async {
    try {
      final result = await BridgeService.instance.playTrack(
        track.id,
        track: track,
      );
      _inflight.remove(track.id);
      if (result.error != null) return null;
      return SourceCache(result.filePath, result.headers);
    } catch (_) {
      _inflight.remove(track.id);
      return null;
    }
  }

  void _attachSource(String trackId, SourceCache cache) {
    final q = queue.value;
    var changed = false;
    final newItems = List<QueueItem>.of(q.items);
    for (var i = 0; i < newItems.length; i++) {
      final item = newItems[i];
      if (item.track.id == trackId && item.source == null) {
        newItems[i] = item.copyWith(source: cache);
        changed = true;
      }
    }
    if (changed) {
      queue.value = QueueState(items: newItems, currentIndex: q.currentIndex);
    }

    var orderedChanged = false;
    final newOrdered = List<QueueItem>.of(_orderedItems);
    for (var i = 0; i < newOrdered.length; i++) {
      final item = newOrdered[i];
      if (item.track.id == trackId && item.source == null) {
        newOrdered[i] = item.copyWith(source: cache);
        orderedChanged = true;
      }
    }
    if (orderedChanged) {
      _orderedItems = newOrdered;
    }
  }

  AudioSource _buildSource(TrackInfo track, SourceCache cache) {
    if (isUrl(cache.filePath)) {
      final headers = cache.headers?.fold<Map<String, String>>({}, (m, pair) {
        m[pair[0]] = pair[1];
        return m;
      });
      return AudioSource.uri(
        Uri.parse(cache.filePath),
        headers: headers,
        tag: track.id,
      );
    }
    return AudioSource.file(cache.filePath, tag: track.id);
  }

  void _beginTrack(TrackInfo track) {
    _trackStartTime = DateTime.now();
    _updateNowPlaying(track);
  }

  void _updateNowPlaying(TrackInfo track) {
    final userId = SettingsManager.currentUser.value?.id ?? '';
    if (userId.isEmpty) return;
    BridgeService.instance.updateNowPlaying(userId, track.id);
  }

  bool _hasReachedThreshold(TrackInfo track) {
    if (_trackStartTime == null) return false;
    if (track.durationSecs == null || track.durationSecs! <= 0) return false;
    final elapsed = DateTime.now().difference(_trackStartTime!).inSeconds;
    final threshold = (track.durationSecs! * 0.8).toInt();
    return elapsed >= threshold;
  }

  void _scrobbleIfNeeded(TrackInfo? track) {
    if (track == null) return;
    if (!_hasReachedThreshold(track)) return;
    final userId = SettingsManager.currentUser.value?.id ?? '';
    if (userId.isEmpty) return;
    BridgeService.instance.reportPlayback(
      userId,
      track.id,
      _trackStartTime!.toUtc().toIso8601String(),
      track.source,
    );
  }

  Future<void> _prefetchWindow() async {
    final q = queue.value;
    if (q.items.isEmpty) return;
    final len = q.items.length;
    final c = q.currentIndex;
    final wrapping = loopMode.value == LoopMode.all;

    final targets = <int>{};
    for (var d = -_prefetchBehind; d <= _prefetchAhead; d++) {
      if (d == 0) continue;
      if (wrapping) {
        targets.add((c + d) % len);
      } else {
        final i = c + d;
        if (i >= 0 && i < len) targets.add(i);
      }
    }

    final futures = <Future<void>>[];
    for (final i in targets) {
      final item = q.items[i];
      if (item.source == null) {
        futures.add(
          _resolveSource(item.track).then((cache) {
            if (cache != null) _attachSource(item.track.id, cache);
          }),
        );
      }
    }
    await Future.wait(futures);
  }

  Future<void> _restoreAfterPreview() async {
    final saved = _previewRestore;
    _previewRestore = null;

    await _player.stop();
    await _player.clearAudioSources();

    if (saved == null || saved.items.isEmpty) {
      await _reset();
      return;
    }

    queue.value = QueueState(items: saved.items, currentIndex: saved.index);
    _loadedTrackId = null;
    final ok = await _loadTrackAt(saved.index, autoPlay: false);
    if (ok && saved.position > Duration.zero) {
      await _player.seek(saved.position);
      position.value = saved.position;
    }
    if (ok && saved.playing) {
      await _player.play();
    }
  }

  Future<void> _reset() async {
    await _player.stop();
    await _player.clearAudioSources();
    _sourceGeneration++;
    position.value = Duration.zero;
    _previewRestore = null;
    _loadedTrackId = null;
    _orderedItems = const [];
    _inflight.clear();
    playerState.value = PlayerState(false, ProcessingState.idle);
    queue.value = const QueueState();
    shuffleModeEnabled.value = false;
    loopMode.value = LoopMode.off;
  }

  Future<void> _finish() async {
    await _player.stop();
    playerState.value = PlayerState(false, ProcessingState.idle);
    position.value = Duration.zero;
  }
}

class _PreviewRestore {
  final List<QueueItem> items;
  final int index;
  final Duration position;
  final bool playing;

  const _PreviewRestore({
    required this.items,
    required this.index,
    required this.position,
    required this.playing,
  });
}

class SourceCache {
  final String filePath;
  final List<List<String>>? headers;

  const SourceCache(this.filePath, this.headers);
}
