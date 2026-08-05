import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/pages/identify/models/identify_source.dart';
import 'package:tawai/ui/pages/identify/models/identify_session.dart';
import 'package:tawai/ui/pages/identify/models/identify_result.dart';
import 'package:tawai/ui/pages/identify/utils/identify_helpers.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/logger.dart';
import 'package:tawai/utils/settings.dart';

class IdentifyController extends ChangeNotifier {
  IdentifySource selectedSource = const UnidentifiedSource();
  List<LibrarySourceInfo> librarySources = [];
  List<TrackInfo> tracks = [];
  bool loadingTracks = true;
  TrackInfo? selectedTrack;
  final Map<String, IdentifySession> sessions = {};
  final Map<String, IdentifyAlbumResult> albumResults = {};
  final Set<String> _processedTrackIds = {};

  Future<void> loadSources() async {
    final userId = SettingsManager.currentUserId.value;
    if (userId == null || userId.isEmpty) {
      librarySources = [];
      notifyListeners();
      return;
    }
    librarySources = (await BridgeService.instance.listEditableSources(userId))
        .where(
          (s) =>
              s.sourceType != 'jellyfin' &&
              !s.sourceType.startsWith('recommendation:'),
        )
        .toList();
    notifyListeners();
  }

  Future<void> loadTracks() async {
    loadingTracks = true;
    notifyListeners();
    try {
      List<TrackInfo> fetched;
      if (selectedSource is UnidentifiedSource) {
        fetched = await BridgeService.instance.listUnidentifiedTracks();
      } else if (selectedSource is DownloadFolderSource) {
        fetched = await BridgeService.instance.listDownloadFolderTracks(
          path: SettingsManager.downloadFolder.value,
        );
      } else if (selectedSource is LibrarySource) {
        final src = selectedSource as LibrarySource;
        fetched = await BridgeService.instance.listTracksBySource(src.info.id);
      } else {
        fetched = [];
      }
      final identified = fetched
          .where((t) => t.mbidRecording != null && t.albumMbid != null)
          .toList();
      final unidentified = fetched
          .where((t) => !(t.mbidRecording != null && t.albumMbid != null))
          .toList();
      tracks = unidentified
          .where((t) => !_processedTrackIds.contains(t.id))
          .toList();
      groupIdentifiedTracks(identified, albumResults);
      selectedTrack = null;
    } catch (e) {
      log('loadTracks error: $e', isError: true);
    }
    loadingTracks = false;
    notifyListeners();
  }

  Future<void> onSourceChanged(IdentifySource source) async {
    selectedSource = source;
    sessions.clear();
    albumResults.clear();
    _processedTrackIds.clear();
    await loadTracks();
  }

  void onSelectTrack(TrackInfo track) {
    selectedTrack = track;
    notifyListeners();
  }

  Future<RecordingInfo?> fingerprintTrack(TrackInfo track) async {
    final remoteRecording = await BridgeService.instance.fingerprintTrack(
      track.id,
      filePath: track.sourceType == 'download_folder' ? track.filePath : null,
    );
    if (remoteRecording != null) addSessionResult(track, remoteRecording);
    return remoteRecording;
  }

  Future<List<RecordingInfo>> lookupTrack(TrackInfo track) async {
    final parts = <String>[];
    if (track.artistsString.isNotEmpty) {
      parts.add('artist:"${track.artistsString.replaceAll('"', '\\"')}"');
    }
    if (track.title.isNotEmpty) {
      parts.add('recording:"${track.title.replaceAll('"', '\\"')}"');
    }
    final query = parts.join(' AND ');
    final remoteResults = await BridgeService.instance.searchMusicBrainz(query);
    if (remoteResults.length == 1) {
      addSessionResult(track, remoteResults.first);
    }
    return remoteResults;
  }

  void addSessionResult(TrackInfo track, RecordingInfo remoteRecording) {
    final session = IdentifySession(track: track, recording: remoteRecording);
    sessions[track.id] = session;
    _processedTrackIds.add(track.id);
    tracks.removeWhere((t) => t.id == track.id);
    if (selectedTrack?.id == track.id) {
      selectedTrack = null;
    }
    groupSessionIntoAlbum(session, track, albumResults);
    notifyListeners();
  }

  Future<void> onReleaseSelected(
    IdentifySession session,
    ReleaseInfo remoteRelease,
  ) async {
    session.selectedRelease = remoteRelease;
    session.loadingReleaseTracks = true;
    session.releaseTracks = [];
    session.correctTrack = null;
    notifyListeners();
    final remoteReleaseData = await BridgeService.instance.getReleaseTracks(
      remoteRelease.id,
    );
    session.loadingReleaseTracks = false;
    session.releaseTracks = remoteReleaseData.tracks;
    _autoAssignSessionTrack(session);
    final albumKey = IdentifyAlbumResult.albumKey(session.track.albumId);
    final result = albumResults[albumKey];
    if (result != null) {
      if (result.releaseTracks.isEmpty) {
        result.releaseTracks = remoteReleaseData.tracks;
      }
      result.albumMbid = remoteRelease.id;
      result.releaseDate = remoteRelease.date;
      result.releaseArtist = remoteRelease.artist;
      result.releaseArtistMbid = remoteRelease.artistId;
      result.albumDisambiguation = remoteRelease.disambiguation;
    }
    notifyListeners();
  }

  void onTrackSelected(IdentifySession session, ReleaseTrackInfo remoteTrack) {
    session.correctTrack = remoteTrack;
    notifyListeners();
  }

  void onSessionApplied(IdentifySession session) {
    final userTrack = findUserTrack(session.track.id);
    if (userTrack != null) {
      userTrack.applied = true;
      userTrack.isSession = false;
    }
    sessions.remove(session.track.id);
    notifyListeners();
  }

  IdentifiedUserTrack? findUserTrack(String trackId) {
    for (final result in albumResults.values) {
      for (final ut in result.userTracks) {
        if (ut.trackId == trackId) return ut;
      }
    }
    return null;
  }

  void _autoAssignSessionTrack(IdentifySession session) {
    if (session.releaseTracks.isEmpty) {
      session.correctTrack = null;
      return;
    }
    session.correctTrack = autoAssignCorrectTrack(
      trackTitle: session.track.title,
      trackDuration: session.track.durationSecs,
      trackPosition: session.track.trackNum,
      trackDiscNumber: session.track.discNum,
      releaseTracks: session.releaseTracks,
    );
  }

  Future<void> onToggleAlbumExpand(String albumKey) async {
    final result = albumResults[albumKey];
    if (result == null) return;
    if (result.expanded) {
      result.expanded = false;
      notifyListeners();
      return;
    }
    result.expanded = true;
    if (result.releaseTracks.isEmpty && !result.loadingMbid) {
      await _fetchAlbumData(albumKey, result);
    } else {
      notifyListeners();
    }
  }

  Future<void> _fetchAlbumData(
    String albumKey,
    IdentifyAlbumResult result,
  ) async {
    result.loadingMbid = true;
    notifyListeners();
    try {
      String? mbid = result.albumMbid;
      if (mbid == null) {
        final sourceAlbumId = result.sourceAlbumId;
        if (sourceAlbumId == null) throw StateError('no source');
        mbid = await BridgeService.instance.getAlbumMbid(sourceAlbumId);
        if (mbid == null) throw StateError('no mbid');
        result.albumMbid = mbid;
      }
      final remoteRelease = await BridgeService.instance.getReleaseTracks(mbid);
      result.releaseTracks = remoteRelease.tracks;
      result.albumTitle = remoteRelease.releaseTitle;
      result.releaseDate = remoteRelease.releaseDate;
      result.releaseArtist = remoteRelease.artist;
      result.releaseArtistMbid = remoteRelease.artistId;
      result.albumDisambiguation = remoteRelease.disambiguation;
      for (final ut in result.userTracks.where(
        (t) => t.isSession && t.sessionId != null,
      )) {
        final s = sessions[ut.sessionId!];
        if (s != null && s.correctTrack == null) {
          s.releaseTracks = remoteRelease.tracks;
          _autoAssignSessionTrack(s);
        }
      }
    } catch (_) {}
    result.loadingMbid = false;
    notifyListeners();
  }

  Future<RecordingInfo?> fetchRecordingForTrack(IdentifiedUserTrack ut) async {
    if (ut.isSession && ut.sessionId != null) {
      final session = sessions[ut.sessionId];
      return session?.recording;
    }
    if (ut.mbidRecording == null) return null;
    return BridgeService.instance.fetchRecording(ut.mbidRecording!);
  }

  void setTrackLyrics(String albumKey, String trackId, String lyrics) {
    final result = albumResults[albumKey];
    if (result == null) return;
    final idx = result.userTracks.indexWhere((t) => t.trackId == trackId);
    if (idx < 0) return;
    result.userTracks[idx] = IdentifiedUserTrack(
      trackId: result.userTracks[idx].trackId,
      title: result.userTracks[idx].title,
      artistsString: result.userTracks[idx].artistsString,
      albumTitle: result.userTracks[idx].albumTitle,
      trackNum: result.userTracks[idx].trackNum,
      discNum: result.userTracks[idx].discNum,
      artists: result.userTracks[idx].artists,
      lyrics: lyrics,
      mbidRecording: result.userTracks[idx].mbidRecording,
      isSession: result.userTracks[idx].isSession,
      sessionId: result.userTracks[idx].sessionId,
      applied: result.userTracks[idx].applied,
      assignedPosition: result.userTracks[idx].assignedPosition,
    );
    notifyListeners();
  }

  Future<void> updateAlbumRelease(String albumKey, String releaseId) async {
    final result = albumResults[albumKey];
    if (result == null) return;
    final remoteRelease = await BridgeService.instance.getReleaseTracks(
      releaseId,
    );
    result.releaseTracks = remoteRelease.tracks;
    result.albumMbid = releaseId;
    result.albumTitle = remoteRelease.releaseTitle;
    result.releaseDate = remoteRelease.releaseDate;
    result.releaseArtist = remoteRelease.artist;
    result.releaseArtistMbid = remoteRelease.artistId;
    notifyListeners();
  }

  void updateUserTrackAfterApply({
    required String albumKey,
    required String trackId,
    required String title,
    required String artist,
    required String albumTitle,
    int? trackNum,
    int? discNum,
    String? mbidRecording,
    String? mbidAlbum,
    String? mbidArtist,
    String? lyrics,
    String? releaseDate,
  }) {
    final album = albumResults[albumKey];
    if (album == null) return;
    final idx = album.userTracks.indexWhere((t) => t.trackId == trackId);
    if (idx < 0) return;
    final old = album.userTracks[idx];
    album.userTracks[idx] = IdentifiedUserTrack(
      trackId: old.trackId,
      title: title,
      artistsString: artist,
      albumTitle: albumTitle,
      trackNum: trackNum ?? old.trackNum,
      discNum: discNum ?? old.discNum,
      artists: old.artists,
      lyrics: lyrics ?? old.lyrics,
      mbidRecording: mbidRecording ?? old.mbidRecording,
      isSession: false,
      applied: true,
      assignedPosition: old.assignedPosition,
    );
    if (mbidAlbum != null) album.albumMbid = mbidAlbum;
    if (mbidArtist != null) album.releaseArtistMbid = mbidArtist;
    if (releaseDate != null) album.releaseDate = releaseDate;
    sessions.remove(trackId);
    notifyListeners();
  }

  TrackInfo buildTrackInfoForSelection(
    IdentifiedUserTrack ut,
    IdentifyAlbumResult album, {
    ReleaseTrackInfo? remoteTrack,
  }) {
    return TrackInfo(
      id: ut.trackId,
      title: ut.title,
      albumId: album.sourceAlbumId ?? '',
      albumTitle: ut.albumTitle.isNotEmpty ? ut.albumTitle : album.albumTitle,
      artists: ut.artists,
      artistsString: ut.artistsString.isNotEmpty
          ? ut.artistsString
          : (album.releaseArtist ?? ''),
      trackNum: ut.trackNum,
      discNum: ut.discNum,
      durationSecs: remoteTrack?.durationSecs,
      mbidRecording: ut.mbidRecording ?? remoteTrack?.id,
      albumMbid: album.albumMbid,
      artistMbid: album.releaseArtistMbid,
      releaseDate: album.releaseDate,
      filePath: '',
      source: '',
      sourceType: '',
      genres: const [],
      lyrics: ut.lyrics,
    );
  }

  Future<Uint8List?> downloadCover(String albumMbid) async {
    try {
      final response = await http.get(
        Uri.parse(
          'https://coverartarchive.org/release/$albumMbid/front-250.jpg',
        ),
      );
      if (response.statusCode == 200) return response.bodyBytes;
    } catch (_) {}
    return null;
  }

  Future<Uint8List?> loadLocalCover(String albumId) async {
    try {
      return await BridgeService.instance.getAlbumCover(albumId);
    } catch (_) {
      return null;
    }
  }

  Future<({bool success, String? error, String? newFilePath})>
  applyIdentification({
    required String trackId,
    required String title,
    required String artist,
    String? artistMbid,
    required String album,
    String? albumMbid,
    String? albumDisambiguation,
    String? releaseDate,
    int? trackNum,
    int? discNum,
    String? mbidRecording,
    String? lyrics,
    Uint8List? coverBytes,
    String? filePath,
    String? targetSourceId,
  }) {
    final userId = SettingsManager.currentUserId.value ?? '';
    return BridgeService.instance.applyIdentification(
      userId: userId,
      trackId: trackId,
      title: title,
      artist: artist,
      artistMbid: artistMbid,
      album: album,
      albumMbid: albumMbid,
      albumDisambiguation: albumDisambiguation,
      releaseDate: releaseDate,
      trackNum: trackNum,
      discNum: discNum,
      mbidRecording: mbidRecording,
      lyrics: lyrics,
      coverBytes: coverBytes,
      filePath: filePath,
      targetSourceId: targetSourceId,
    );
  }

  Future<void> applyDownloadScan(String sourceId) async {
    final userId = SettingsManager.currentUserId.value;
    if (userId == null || userId.isEmpty) return;
    await BridgeService.instance.scanSource(
      userId: userId,
      sourceId: sourceId,
      force: false,
    );
  }

  Future<String?> fetchRemoteLyrics({
    required String title,
    required String artist,
    required String? album,
    required double? duration,
    bool preferSync = true,
  }) async {
    try {
      final result = await BridgeService.instance.fetchLyrics(
        title: title,
        artist: artist,
        album: album,
        duration: duration,
        preferSync: preferSync,
      );
      if (result != null && result.lyrics.isNotEmpty) return result.lyrics;
    } catch (_) {}
    return null;
  }

  void refresh() {
    notifyListeners();
  }

  void onStartEditingTrack(String albumKey, String userTrackId) {
    final result = albumResults[albumKey];
    if (result != null) {
      result.editingTrackId = userTrackId;
      notifyListeners();
    }
  }

  void onStopEditingTrack(String albumKey) {
    final result = albumResults[albumKey];
    if (result != null) {
      result.editingTrackId = null;
      notifyListeners();
    }
  }

  void onAssignPosition(
    String albumKey,
    String editingTrackId,
    int newPosition,
  ) {
    final result = albumResults[albumKey];
    if (result == null) return;
    final editingTrack = result.userTracks
        .where((t) => t.trackId == editingTrackId)
        .firstOrNull;
    if (editingTrack == null) return;
    final swapTrack = result.userTracks
        .where((t) => t.assignedPosition == newPosition)
        .firstOrNull;
    final oldPosition = editingTrack.assignedPosition;
    editingTrack.assignedPosition = newPosition;
    if (swapTrack != null && swapTrack.trackId != editingTrackId) {
      swapTrack.assignedPosition = oldPosition;
    }
    result.editingTrackId = null;
    notifyListeners();
  }
}
