import 'dart:async';

import 'package:audio_service/audio_service.dart';
import 'package:flutter/foundation.dart';
import 'package:just_audio/just_audio.dart';

import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/services/playback_service.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/io_service.dart';
import 'package:tawai/utils/logger.dart';

/// Bridges [PlaybackService] to `audio_service` so that playback can be
/// controlled from the Android media notification / lock screen and, on web,
/// from the browser Media Session API.
///
/// [PlaybackService] remains the single source of truth for playback state;
/// this handler merely observes its notifiers and forwards system requests
/// back into it.
class TawaiAudioHandler extends BaseAudioHandler with SeekHandler {
  TawaiAudioHandler() {
    attach();
  }

  final PlaybackService _ps = PlaybackService.instance;
  final Map<String, Uri> _artCache = {};
  bool _attached = false;
  int _artGeneration = 0;

  /// Subscribes to [PlaybackService] notifiers and broadcasts their state to
  /// `audio_service` clients. Idempotent.
  void attach() {
    if (_attached) return;
    _attached = true;

    _ps.queue.addListener(_broadcastQueue);
    _ps.playerState.addListener(_broadcastState);
    _ps.shuffleModeEnabled.addListener(_broadcastState);
    _ps.loopMode.addListener(_broadcastState);

    _broadcastQueue();
    _broadcastState();
  }

  // ---------------------------------------------------------------------------
  // State broadcast
  // ---------------------------------------------------------------------------

  void _broadcastQueue() {
    final q = _ps.queue.value;
    final items = [for (final item in q.items) _toMediaItem(item.track)];
    queue.add(items);
    final current = q.currentTrack;
    if (current == null) {
      mediaItem.add(null);
      return;
    }
    final item = _toMediaItem(current);
    mediaItem.add(item);
    unawaited(_resolveArt(current, item));
  }

  void _broadcastState() {
    final state = _ps.playerState.value;
    final playing = state.playing;
    playbackState.add(
      playbackState.value.copyWith(
        controls: [
          MediaControl.skipToPrevious,
          if (playing) MediaControl.pause else MediaControl.play,
          MediaControl.stop,
          MediaControl.skipToNext,
        ],
        systemActions: const {
          MediaAction.seek,
          MediaAction.seekForward,
          MediaAction.seekBackward,
        },
        androidCompactActionIndices: const [0, 1, 3],
        processingState: _mapProcessingState(state.processingState),
        playing: playing,
        updatePosition: _ps.position.value,
        speed: 1.0,
        queueIndex: _ps.queue.value.currentIndex,
        repeatMode: _mapRepeatMode(_ps.loopMode.value),
        shuffleMode: _ps.shuffleModeEnabled.value
            ? AudioServiceShuffleMode.all
            : AudioServiceShuffleMode.none,
      ),
    );
  }

  /// Resolves cover art for [track] and updates the broadcast media item once
  /// it is ready. Guarded against stale responses via [_artGeneration].
  Future<void> _resolveArt(TrackInfo track, MediaItem item) async {
    final gen = ++_artGeneration;
    Uri? art;
    try {
      art = await _resolveArtUri(track.id);
    } catch (e) {
      log('resolveArt error: ${track.id}: $e', isError: true);
    }
    if (art == null) return;
    if (gen != _artGeneration) return;
    if (_ps.queue.value.currentTrack?.id != track.id) return;
    _artCache[track.id] = art;
    final updated = item.copyWith(artUri: art);
    mediaItem.add(updated);
    final q = _ps.queue.value;
    final updatedQueue = [
      for (final qi in q.items)
        if (qi.track.id == track.id)
          _toMediaItem(qi.track).copyWith(artUri: art)
        else
          _toMediaItem(qi.track),
    ];
    queue.add(updatedQueue);
  }

  /// Fetches the cover for [trackId] as bytes from the bridge and exposes it
  /// as a URI the system can load: a data URI on web, a local file otherwise.
  Future<Uri?> _resolveArtUri(String trackId) async {
    final cached = _artCache[trackId];
    if (cached != null) return cached;

    final bytes = await BridgeService.instance.getTrackCover(trackId);
    if (bytes == null || bytes.isEmpty) return null;

    if (kIsWeb) {
      return Uri.dataFromBytes(bytes, mimeType: 'image/jpeg');
    }

    final io = IOServiceFactory.create();
    final dir = await io.getTempDir();
    final path = '$dir/covers/$trackId.jpg';
    if (!await io.fileExists(path)) {
      await io.createDirectory('$dir/covers', recursive: true);
      await io.writeFileBytes(path, bytes);
    }
    return Uri.file(path);
  }

  MediaItem _toMediaItem(TrackInfo track) {
    return MediaItem(
      id: track.id,
      title: track.title,
      artist: track.artistsString,
      album: track.albumTitle.isEmpty ? null : track.albumTitle,
      duration: track.durationSecs != null
          ? Duration(milliseconds: (track.durationSecs! * 1000).round())
          : null,
      artUri: _artCache[track.id],
    );
  }

  AudioProcessingState _mapProcessingState(ProcessingState state) {
    switch (state) {
      case ProcessingState.idle:
        return AudioProcessingState.idle;
      case ProcessingState.loading:
        return AudioProcessingState.loading;
      case ProcessingState.buffering:
        return AudioProcessingState.buffering;
      case ProcessingState.ready:
        return AudioProcessingState.ready;
      case ProcessingState.completed:
        return AudioProcessingState.completed;
    }
  }

  AudioServiceRepeatMode _mapRepeatMode(LoopMode mode) {
    switch (mode) {
      case LoopMode.off:
        return AudioServiceRepeatMode.none;
      case LoopMode.all:
        return AudioServiceRepeatMode.all;
      case LoopMode.one:
        return AudioServiceRepeatMode.one;
    }
  }

  LoopMode _mapLoopMode(AudioServiceRepeatMode mode) {
    switch (mode) {
      case AudioServiceRepeatMode.none:
        return LoopMode.off;
      case AudioServiceRepeatMode.all:
        return LoopMode.all;
      case AudioServiceRepeatMode.one:
        return LoopMode.one;
      case AudioServiceRepeatMode.group:
        return LoopMode.all;
    }
  }

  // ---------------------------------------------------------------------------
  // System callbacks
  // ---------------------------------------------------------------------------

  @override
  Future<void> play() => _ps.resume();

  @override
  Future<void> pause() => _ps.pause();

  @override
  Future<void> seek(Duration position) => _ps.seek(position);

  @override
  Future<void> stop() => _ps.clearQueue();

  @override
  Future<void> skipToNext() => _ps.playNext();

  @override
  Future<void> skipToPrevious() => _ps.playPrevious();

  @override
  Future<void> skipToQueueItem(int index) => _ps.playTrackAt(index);

  @override
  Future<void> setRepeatMode(AudioServiceRepeatMode repeatMode) =>
      _ps.setLoopMode(_mapLoopMode(repeatMode));

  @override
  Future<void> setShuffleMode(AudioServiceShuffleMode shuffleMode) =>
      _ps.toggleShuffle(force: shuffleMode == AudioServiceShuffleMode.all);

  @override
  Future<void> removeQueueItemAt(int index) => _ps.removeFromQueue(index);

  Future<void> moveQueueItem(int currentIndex, int newIndex) =>
      _ps.moveQueueItem(currentIndex, newIndex);

  @override
  Future<void> onTaskRemoved() async {
    if (!_ps.playerState.value.playing) {
      await _ps.clearQueue();
    }
  }
}
