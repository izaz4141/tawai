import 'dart:async';
import 'package:flutter/foundation.dart';

import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/utils/platform_service.dart';
import 'package:tawai/utils/api_service.dart';
import 'package:tawai/utils/rinf_service.dart';
import 'package:tawai/utils/settings.dart';
import 'package:tawai/ui/pages/search/search_types.dart';

class BridgeService {
  static final BridgeService _instance = BridgeService._();
  static BridgeService get instance => _instance;
  BridgeService._();

  bool get _isRemote => PlatformService().isRemote;

  // ---------------------------------------------------------------------------
  // Crypt
  // ---------------------------------------------------------------------------

  Future<String?> decrypt(String encryptedKey, {String? masterKey}) async {
    return _isRemote
        ? APIService.instance.decrypt(encryptedKey)
        : RinfService.instance.decrypt(encryptedKey, masterKey: masterKey);
  }

  Future<String?> encrypt(String plainKey, {String? masterKey}) async {
    return _isRemote
        ? APIService.instance.encrypt(plainKey)
        : RinfService.instance.encrypt(plainKey, masterKey: masterKey);
  }

  Future<({String encryptedApiKey, String decryptedApiKey, String masterKey})>
  requestNewApiKey({String? masterKey, required String userId}) async {
    return RinfService.instance.requestNewApiKey(
      masterKey: masterKey,
      userId: userId,
    );
  }

  Future<String?> generateMasterKey() async {
    if (_isRemote) {
      return APIService.instance.generateMasterKey();
    }
    return RinfService.instance.generateMasterKey();
  }

  Future<({bool success, String userId, String username})> login(
    String username,
    String password,
  ) async {
    if (_isRemote) {
      final success = await APIService.instance.login(
        username: username,
        password: password,
      );
      final user = SettingsManager.currentUser.value;
      return (
        success: success,
        userId: user?.id ?? '',
        username: user?.username ?? username,
      );
    }
    return RinfService.instance.login(username, password, username, password);
  }

  Future<
    ({
      bool success,
      String userId,
      String username,
      String displayName,
      String role,
      String apiKey,
    })
  >
  updateAccount({
    required String operatorUserId,
    required String operatorPassword,
    required String targetUserId,
    String? newUsername,
    String? newPassword,
    String? newDisplayName,
    String? role,
  }) async {
    if (_isRemote) {
      return APIService.instance.updateAccount(
        operatorUserId: operatorUserId,
        operatorPassword: operatorPassword,
        targetUserId: targetUserId,
        newUsername: newUsername,
        newPassword: newPassword,
        newDisplayName: newDisplayName,
        role: role,
      );
    }
    return RinfService.instance.updateAccount(
      operatorUserId: operatorUserId,
      operatorPassword: operatorPassword,
      targetUserId: targetUserId,
      newUsername: newUsername,
      newPassword: newPassword,
      newDisplayName: newDisplayName,
      role: role,
    );
  }

  Future<
    ({
      bool success,
      String userId,
      String username,
      String displayName,
      String role,
      String apiKey,
    })
  >
  createAccount({
    required String adminUsername,
    required String adminPassword,
    required String username,
    required String password,
    String? displayName,
    String? role,
  }) async {
    if (_isRemote) {
      return APIService.instance.createAccount(
        adminUsername: adminUsername,
        adminPassword: adminPassword,
        username: username,
        password: password,
        displayName: displayName,
        role: role,
      );
    }
    return RinfService.instance.createAccount(
      adminUsername: adminUsername,
      adminPassword: adminPassword,
      username: username,
      password: password,
      displayName: displayName,
      role: role,
    );
  }

  Future<List<UserListItem>> listUsers() async {
    if (_isRemote) {
      return APIService.instance.listUsers();
    }
    return RinfService.instance.listUsers();
  }

  Future<({bool success, String username})> deleteUser({
    required String adminUsername,
    required String adminPassword,
    required String targetUsername,
  }) async {
    if (_isRemote) {
      return APIService.instance.deleteUser(
        adminUsername: adminUsername,
        adminPassword: adminPassword,
        targetUsername: targetUsername,
      );
    }
    return RinfService.instance.deleteUser(
      adminUsername: adminUsername,
      adminPassword: adminPassword,
      targetUsername: targetUsername,
    );
  }

  Future<bool> verifyCurrentPassword(String userId, String password) async {
    if (_isRemote) {
      return APIService.instance.verifyPassword(password);
    }
    return RinfService.instance.verifyCurrentPassword(userId, password);
  }

  // ---------------------------------------------------------------------------
  // Library
  // ---------------------------------------------------------------------------

  Future<List<TrackInfo>> getTracks({String? albumId}) async {
    return _isRemote
        ? APIService.instance.getTracks(albumId: albumId)
        : RinfService.instance.getTracks(albumId: albumId);
  }

  Future<List<AlbumInfo>> getAlbums({String? artistId}) async {
    return _isRemote
        ? APIService.instance.getAlbums(artistId: artistId)
        : RinfService.instance.getAlbums(artistId: artistId);
  }

  Future<List<ArtistInfo>> getArtists() async {
    return _isRemote
        ? APIService.instance.getArtists()
        : RinfService.instance.getArtists();
  }

  Future<List<PlaylistInfo>> getPlaylists() async {
    return _isRemote
        ? APIService.instance.getPlaylists()
        : RinfService.instance.getPlaylists();
  }

  Future<String> createPlaylist(String name) async {
    if (_isRemote) return APIService.instance.createPlaylist(name);
    final userId = SettingsManager.currentUserId.value ?? '';
    return RinfService.instance.createPlaylist(userId, name);
  }

  Future<bool> deletePlaylist(String playlistId) async {
    return _isRemote
        ? APIService.instance.deletePlaylist(playlistId)
        : RinfService.instance.deletePlaylist(playlistId);
  }

  // ---------------------------------------------------------------------------
  // Playlist tracks
  // ---------------------------------------------------------------------------

  Future<List<TrackInfo>> getPlaylistTracks(String playlistId) async {
    return _isRemote
        ? APIService.instance.getPlaylistTracks(playlistId)
        : RinfService.instance.getPlaylistTracks(playlistId);
  }

  Future<bool> addTrackToPlaylist(String playlistId, String trackId) async {
    return _isRemote
        ? APIService.instance.addTrackToPlaylist(playlistId, trackId)
        : RinfService.instance.addTrackToPlaylist(playlistId, trackId);
  }

  Future<bool> removeTrackFromPlaylist(
    String playlistId,
    String trackId,
  ) async {
    return _isRemote
        ? APIService.instance.removeTrackFromPlaylist(playlistId, trackId)
        : RinfService.instance.removeTrackFromPlaylist(playlistId, trackId);
  }

  Future<bool> reorderPlaylistTracks(
    String playlistId,
    List<String> trackIds,
  ) async {
    return _isRemote
        ? APIService.instance.reorderPlaylistTracks(playlistId, trackIds)
        : RinfService.instance.reorderPlaylistTracks(playlistId, trackIds);
  }

  // ---------------------------------------------------------------------------
  // Playback
  // ---------------------------------------------------------------------------

  Future<({String filePath, String? error, List<List<String>>? headers})>
  playTrack(String? trackId, {TrackInfo? track}) async {
    if (_isRemote) {
      return APIService.instance.playTrack(trackId, track: track);
    }
    return RinfService.instance.playTrack(trackId, track: track);
  }

  previewTrack(TrackInfo track) async {
    if (_isRemote) {
      return APIService.instance.previewTrack(track);
    }
    return RinfService.instance.previewTrack(track);
  }

  Future<TrackInfo?> getTrackInfo(String trackId) async {
    return _isRemote
        ? APIService.instance.getTrackInfo(trackId)
        : RinfService.instance.getTrackInfo(trackId);
  }

  Future<bool> reportPlayback(
    String userId,
    String trackId,
    String playedAt,
    String source,
  ) async {
    return _isRemote
        ? APIService.instance.reportPlayback(
            trackId: trackId,
            playedAt: playedAt,
            source: source,
          )
        : RinfService.instance.reportPlayback(
            userId,
            trackId,
            playedAt,
            source,
          );
  }

  Future<List<PlaybackRecord>> getHistory(
    String userId, {
    int limit = 50,
  }) async {
    return _isRemote
        ? APIService.instance.getHistory(limit: limit)
        : RinfService.instance.getHistory(userId, limit: limit);
  }

  // ---------------------------------------------------------------------------
  // Downloads (generic)
  // ---------------------------------------------------------------------------

  Future<List<DownloadRecord>> listDownloads({String? source}) async {
    if (_isRemote) {
      return APIService.instance.listDownloads(source: source);
    }
    final userId = SettingsManager.currentUserId.value ?? '';
    if (userId.isEmpty) return [];
    return RinfService.instance.listDownloads(userId, source: source);
  }

  Future<List<DownloadRecord>> pollDownloads() async {
    if (_isRemote) {
      return APIService.instance.pollDownloads();
    }
    final userId = SettingsManager.currentUserId.value ?? '';
    if (userId.isEmpty) return [];
    return RinfService.instance.pollDownloads(userId);
  }

  Future<List<SearchResultItem>> search(String sourceType, String query) async {
    if (_isRemote) {
      final response = await APIService.instance.search(sourceType, query);
      return response.results
          .map((r) => SearchResultItem.fromDlSearchItem(r))
          .toList();
    }
    final response = await RinfService.instance.search(sourceType, query);
    return response.results
        .map((r) => SearchResultItem.fromDlSearchItem(r))
        .toList();
  }

  Future<({bool success, String downloadId, String? error})> create(
    String sourceType,
    String url,
    String dest,
    String userId, {
    String? extra,
  }) async {
    if (_isRemote) {
      return APIService.instance.create(
        sourceType,
        url,
        dest,
        userId,
        extra: extra,
      );
    }
    return RinfService.instance.create(
      sourceType,
      url,
      dest,
      userId,
      extra: extra,
    );
  }

  Future<({bool success, String? error})> cancel(
    String sourceType,
    String downloadId, {
    String? extra,
  }) async {
    if (_isRemote) {
      return APIService.instance.cancel(sourceType, downloadId, extra: extra);
    }
    return RinfService.instance.cancel(sourceType, downloadId, extra: extra);
  }

  Future<({bool success, String? version, String? error})> testConnection(
    String sourceType, {
    String? url,
    String? token,
    String? username,
    String? password,
  }) async {
    if (_isRemote) {
      return APIService.instance.testConnection(
        sourceType,
        url: url,
        token: token,
        username: username,
        password: password,
      );
    }
    return RinfService.instance.testConnection(
      sourceType,
      url: url,
      token: token,
      username: username,
      password: password,
    );
  }

  Future<String?> getCurrentVersion(String app) async {
    if (_isRemote) {
      return APIService.instance.getCurrentVersion(app);
    }
    return RinfService.instance.getCurrentVersion(app);
  }

  Future<
    ({
      String? version,
      String? tagName,
      String? releaseNotes,
      String? publishedAt,
      String? error,
    })
  >
  getLatestVersion(
    String owner,
    String repo, {
    bool nightly = false,
    bool atomic = true,
  }) async {
    if (_isRemote) {
      final info = await APIService.instance.getLatestVersion(
        owner,
        repo,
        nightly: nightly,
        atomic: atomic,
      );
      return (
        version: info?.version,
        tagName: info?.tagName,
        releaseNotes: info?.releaseNotes,
        publishedAt: info?.publishedAt,
        error: null,
      );
    }
    return RinfService.instance.getLatestVersion(
      owner,
      repo,
      nightly: nightly,
      atomic: atomic,
    );
  }

  Future<String?> compareVersions(List<String> versions) async {
    if (_isRemote) {
      return APIService.instance.compareVersions(versions);
    }
    return RinfService.instance.compareVersions(versions);
  }

  Future<String?> getInfo(String sourceType, String url) async {
    if (_isRemote) {
      return APIService.instance.getInfo(sourceType, url);
    }
    final response = await RinfService.instance.getInfo(sourceType, url);
    return response.success ? response.info : null;
  }

  // ---------------------------------------------------------------------------
  // Settings
  // ---------------------------------------------------------------------------

  Future<bool> regenerateApiKey({required String userId}) async {
    if (_isRemote) return APIService.instance.regenerateApiKey();
    await RinfService.instance.requestNewApiKey(
      masterKey: null,
      userId: userId,
    );
    return true;
  }

  Future<Map<String, dynamic>?> getSettings() async {
    if (_isRemote) return APIService.instance.getSettings();
    final result = await RinfService.instance.initConfig(
      SettingsManager.configPath,
    );
    return result.settings;
  }

  Future<Map<String, dynamic>?> getGlobalSettings() async {
    if (_isRemote) return APIService.instance.getSettings();
    return RinfService.instance.getGlobalSettings();
  }

  Future<bool> saveSettings(Map<String, dynamic> settings) async {
    if (_isRemote) return APIService.instance.saveSettings(settings);
    final result = await RinfService.instance.saveConfig(settings);
    return result.success;
  }

  Future<bool> restartServer() async {
    return _isRemote
        ? APIService.instance.restartServer()
        : _localRestartServer();
  }

  Future<bool> setUserSetting(String userId, String key, String value) async {
    if (_isRemote) {
      return APIService.instance.saveSettings({key: value});
    }
    return RinfService.instance.setUserSetting(userId, key, value);
  }

  Future<String> getUserSetting(String userId, String key) async {
    if (_isRemote) {
      final settings = await APIService.instance.getSettings();
      return settings?[key]?.toString() ?? '';
    }
    return RinfService.instance.getUserSetting(userId, key);
  }

  Future<Map<String, String>> getAllUserSettings(String userId) async {
    if (_isRemote) {
      return APIService.instance.getAllUserSettings();
    }
    return RinfService.instance.getAllUserSettings(userId);
  }

  // ---------------------------------------------------------------------------
  // Scan
  // ---------------------------------------------------------------------------

  Future<({bool started, String? error})> scanLibrary({
    required String userId,
    required bool force,
  }) {
    return _isRemote
        ? APIService.instance.scanLibrary(force: force)
        : RinfService.instance.scanLibrary(userId: userId, force: force);
  }

  Future<({bool started, String? error})> scanSource({
    required String userId,
    required String sourceId,
    required bool force,
  }) {
    return _isRemote
        ? APIService.instance.scanSource(sourceId: sourceId, force: force)
        : RinfService.instance.scanSource(
            userId: userId,
            sourceId: sourceId,
            force: force,
          );
  }

  Future<({bool running, ScanProgressSignal? progress})> getScanStatus() {
    return _isRemote
        ? APIService.instance.getScanStatus()
        : RinfService.instance.getScanStatus();
  }

  // ---------------------------------------------------------------------------
  // Library Sources
  // ---------------------------------------------------------------------------

  Future<List<LibrarySourceInfo>> listLibrarySources(String userId) async {
    if (_isRemote) {
      return APIService.instance.listLibrarySources();
    }
    return RinfService.instance.listLibrarySources(userId);
  }

  Future<List<LibrarySourceInfo>> listEditableSources(String userId) async {
    if (_isRemote) {
      return APIService.instance.listEditableSources();
    }
    return RinfService.instance.listEditableSources(userId);
  }

  Future<({String sourceId, bool success})> addLibrarySource(
    String userId,
    String url,
    String name,
    String sourceType,
  ) async {
    if (_isRemote) {
      return APIService.instance.addLibrarySource(url, name, sourceType);
    }
    return RinfService.instance.addLibrarySource(userId, url, name, sourceType);
  }

  Future<bool> removeLibrarySource(String userId, String sourceId) async {
    if (_isRemote) {
      return APIService.instance.removeLibrarySource(sourceId);
    }
    return RinfService.instance.removeLibrarySource(userId, sourceId);
  }

  Future<List<JellyfinLibraryInfo>> testJellyfinSource(String url) async {
    if (_isRemote) {
      return APIService.instance.testJellyfinSource(url);
    }
    return RinfService.instance.testJellyfinSource(url);
  }

  // ---------------------------------------------------------------------------
  // System (local-only)
  // ---------------------------------------------------------------------------

  Future<bool> initDatabase(String path, {required String masterKey}) async {
    if (!kIsWeb) {
      return RinfService.instance.initDatabase(path, masterKey: masterKey);
    }
    return true;
  }

  void startServer({
    required int port,
    required String masterKey,
    required String configPath,
  }) {
    if (!kIsWeb) {
      RinfService.instance.startServer(
        port: port,
        masterKey: masterKey,
        configPath: configPath,
      );
    }
  }

  Stream<LogSignal> get logSignal =>
      _isRemote ? const Stream.empty() : RinfService.instance.logSignal;

  // ---------------------------------------------------------------------------
  // Cover
  // ---------------------------------------------------------------------------

  Future<Uint8List?> getAlbumCover(String albumId) {
    return _isRemote
        ? APIService.instance.getAlbumCover(albumId)
        : RinfService.instance.getAlbumCover(albumId);
  }

  Future<Uint8List?> getTrackCover(String trackId) {
    return _isRemote
        ? APIService.instance.getTrackCover(trackId)
        : RinfService.instance.getTrackCover(trackId);
  }

  // ---------------------------------------------------------------------------
  // Identify
  // ---------------------------------------------------------------------------

  Future<List<TrackInfo>> listUnidentifiedTracks({String? sourceId}) async {
    return _isRemote
        ? APIService.instance.listUnidentifiedTracks(sourceId: sourceId)
        : RinfService.instance.listUnidentifiedTracks(sourceId: sourceId);
  }

  Future<List<TrackInfo>> listDownloadFolderTracks({String? path}) async {
    return _isRemote
        ? APIService.instance.listDownloadFolderTracks()
        : RinfService.instance.listDownloadFolderTracks(path ?? '');
  }

  Future<List<TrackInfo>> listTracksBySource(String sourceId) async {
    return _isRemote
        ? APIService.instance.listTracksBySource(sourceId)
        : RinfService.instance.listTracksBySource(sourceId);
  }

  Future<String?> getAlbumMbid(String albumId) async {
    return _isRemote
        ? APIService.instance.getAlbumMbid(albumId)
        : RinfService.instance.getAlbumMbid(albumId);
  }

  Future<List<MatchCandidate>> identifySingleTrack(String trackId) async {
    return _isRemote
        ? APIService.instance.identifySingleTrack(trackId)
        : RinfService.instance.identifySingleTrack(trackId);
  }

  Future<List<RecordingInfo>> searchMusicBrainz(String query) async {
    return _isRemote
        ? APIService.instance.searchMusicBrainz(query)
        : RinfService.instance.searchMusicBrainz(query);
  }

  Future<({bool success, String? error, String? newFilePath})>
  applyIdentification({
    required String userId,
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
    List<int>? coverBytes,
    int totalDiscs = 0,
    String? filePath,
    String? targetSourceId,
  }) async {
    return _isRemote
        ? APIService.instance.applyIdentification(
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
            totalDiscs: totalDiscs,
            filePath: filePath,
            targetSourceId: targetSourceId,
          )
        : RinfService.instance.applyIdentification(
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
            totalDiscs: totalDiscs,
            filePath: filePath,
            targetSourceId: targetSourceId,
          );
  }

  // ---------------------------------------------------------------------------
  // Fingerprint track
  // ---------------------------------------------------------------------------

  Future<RecordingInfo?> fingerprintTrack(
    String? trackId, {
    String? filePath,
  }) async {
    return _isRemote
        ? APIService.instance.fingerprintTrack(trackId, filePath: filePath)
        : RinfService.instance.fingerprintTrack(trackId, filePath: filePath);
  }

  Future<GetReleaseTracksResponse> getReleaseTracks(String releaseId) async {
    return _isRemote
        ? APIService.instance.getReleaseTracks(releaseId)
        : RinfService.instance.getReleaseTracks(releaseId);
  }

  Future<RecordingInfo?> fetchRecording(String mbid) async {
    return _isRemote
        ? APIService.instance.fetchRecording(mbid)
        : RinfService.instance.fetchRecording(mbid);
  }

  // ---------------------------------------------------------------------------
  // Tag editor (standalone file tag reader/writer, local-only)
  // ---------------------------------------------------------------------------

  Future<ReadFileTagsResponse?> readFileTags(String path) {
    if (_isRemote) return APIService.instance.readFileTags(path);
    return RinfService.instance.readFileTags(path);
  }

  Future<({bool success, String? error})?> writeFileTags({
    required String path,
    required String title,
    required String artist,
    required String album,
    required String albumArtist,
    required List<String> genres,
    required int trackNumber,
    required int discNumber,
    String? releaseDate,
    String? lyrics,
    List<int>? cover,
  }) {
    if (_isRemote) {
      return APIService.instance.writeFileTags(
        path: path,
        title: title,
        artist: artist,
        album: album,
        albumArtist: albumArtist,
        genres: genres,
        trackNumber: trackNumber,
        discNumber: discNumber,
        releaseDate: releaseDate,
        lyrics: lyrics,
        cover: cover,
      );
    }
    return RinfService.instance.writeFileTags(
      path: path,
      title: title,
      artist: artist,
      album: album,
      albumArtist: albumArtist,
      genres: genres,
      trackNumber: trackNumber,
      discNumber: discNumber,
      releaseDate: releaseDate,
      lyrics: lyrics,
      cover: cover,
    );
  }

  Future<ReadFileTagsResponse?> readFileTagsBytes(
    String filename,
    Uint8List bytes,
  ) {
    if (_isRemote)
      return APIService.instance.readFileTagsBytes(filename, bytes);
    return RinfService.instance.readFileTagsBytes(filename, bytes);
  }

  Future<({bool success, Uint8List? bytes, String? error})?>
  writeFileTagsBytes({
    required String filename,
    required Uint8List bytes,
    required String title,
    required String artist,
    required String album,
    required String albumArtist,
    required List<String> genres,
    required int trackNumber,
    required int discNumber,
    String? releaseDate,
    String? lyrics,
    List<int>? cover,
  }) {
    if (_isRemote) {
      return APIService.instance.writeFileTagsBytes(
        filename: filename,
        bytes: bytes,
        title: title,
        artist: artist,
        album: album,
        albumArtist: albumArtist,
        genres: genres,
        trackNumber: trackNumber,
        discNumber: discNumber,
        releaseDate: releaseDate,
        lyrics: lyrics,
        cover: cover,
      );
    }
    return RinfService.instance.writeFileTagsBytes(
      filename: filename,
      bytes: bytes,
      title: title,
      artist: artist,
      album: album,
      albumArtist: albumArtist,
      genres: genres,
      trackNumber: trackNumber,
      discNumber: discNumber,
      releaseDate: releaseDate,
      lyrics: lyrics,
      cover: cover,
    );
  }

  // ---------------------------------------------------------------------------
  // Tagging (local-only)
  // ---------------------------------------------------------------------------

  Future<String> createTagging(String name, String description) async {
    if (_isRemote) return '';
    return RinfService.instance.createTagging(name, description);
  }

  Future<String> putTagging(String name, String tag) async {
    if (_isRemote) return '';
    return RinfService.instance.putTagging(name, tag);
  }

  // ---------------------------------------------------------------------------
  // Fetch lyrics
  // ---------------------------------------------------------------------------

  Future<LyricsResult?> fetchLyrics({
    required String title,
    required String artist,
    String? album,
    double? duration,
    bool preferSync = true,
  }) async {
    return _isRemote
        ? APIService.instance.fetchLyrics(
            title: title,
            artist: artist,
            album: album,
            duration: duration,
            preferSync: preferSync,
          )
        : RinfService.instance.fetchLyrics(
            title: title,
            artist: artist,
            album: album,
            duration: duration,
            preferSync: preferSync,
          );
  }

  Future<List<LyricsResult>> searchLyrics({
    required String query,
    bool preferSync = true,
  }) async {
    return _isRemote
        ? APIService.instance.searchLyrics(query: query, preferSync: preferSync)
        : RinfService.instance.searchLyrics(
            query: query,
            preferSync: preferSync,
          );
  }

  // ---------------------------------------------------------------------------
  // Tools
  // ---------------------------------------------------------------------------

  // ---------------------------------------------------------------------------
  // Naming format preview
  // ---------------------------------------------------------------------------

  Future<String> formatNamingPreview({
    required String pattern,
    required String title,
    required String artist,
    required String albumArtist,
    required String album,
    String? releaseDate,
    required int trackNumber,
    required int discNumber,
    String? albumDisambiguation,
    int totalDiscs = 0,
  }) async {
    if (_isRemote) {
      return APIService.instance.formatNamingPreview(
        pattern: pattern,
        title: title,
        artist: artist,
        albumArtist: albumArtist,
        album: album,
        releaseDate: releaseDate,
        trackNumber: trackNumber,
        discNumber: discNumber,
        albumDisambiguation: albumDisambiguation,
        totalDiscs: totalDiscs,
      );
    }
    return RinfService.instance.formatNamingPreview(
      pattern: pattern,
      title: title,
      artist: artist,
      albumArtist: albumArtist,
      album: album,
      releaseDate: releaseDate,
      trackNumber: trackNumber,
      discNumber: discNumber,
      albumDisambiguation: albumDisambiguation,
      totalDiscs: totalDiscs,
    );
  }

  // ---------------------------------------------------------------------------
  // Rename
  // ---------------------------------------------------------------------------

  Future<BatchRenamePreviewResponse?> batchRenamePreview({
    required List<String> filePaths,
    required String pattern,
    String? sourceId,
  }) async {
    if (_isRemote) {
      return APIService.instance.batchRenamePreview(
        filePaths: filePaths,
        pattern: pattern,
        sourceId: sourceId,
      );
    }
    return RinfService.instance.batchRenamePreview(
      filePaths: filePaths,
      pattern: pattern,
      sourceId: sourceId,
    );
  }

  Future<BatchRenameApplyResponse?> batchRenameApply({
    required List<String> filePaths,
    required List<String> trackIds,
    required String pattern,
  }) async {
    if (_isRemote) {
      return APIService.instance.batchRenameApply(
        filePaths: filePaths,
        trackIds: trackIds,
        pattern: pattern,
      );
    }
    return RinfService.instance.batchRenameApply(
      filePaths: filePaths,
      trackIds: trackIds,
      pattern: pattern,
    );
  }

  Future<CheckNamingConventionResponse?> checkNamingConvention({
    String? sourceId,
    required String pattern,
  }) async {
    if (_isRemote) {
      return APIService.instance.checkNamingConvention(
        sourceId: sourceId,
        pattern: pattern,
      );
    }
    return RinfService.instance.checkNamingConvention(
      sourceId: sourceId,
      pattern: pattern,
    );
  }

  Future<GetLibraryStatsResponse?> getLibraryStats({
    String? namingPattern,
  }) async {
    if (_isRemote) {
      return APIService.instance.getLibraryStats(namingPattern: namingPattern);
    }
    return RinfService.instance.getLibraryStats(namingPattern: namingPattern);
  }

  Future<FindMissingMetadataResponse?> findMissingMetadata({
    required bool checkTitle,
    required bool checkArtist,
    required bool checkAlbum,
    required bool checkGenre,
    required bool checkYear,
    required bool checkTrackNumber,
    required bool checkCover,
  }) async {
    if (_isRemote) {
      return APIService.instance.findMissingMetadata(
        checkTitle: checkTitle,
        checkArtist: checkArtist,
        checkAlbum: checkAlbum,
        checkGenre: checkGenre,
        checkYear: checkYear,
        checkTrackNumber: checkTrackNumber,
        checkCover: checkCover,
      );
    }
    return RinfService.instance.findMissingMetadata(
      checkTitle: checkTitle,
      checkArtist: checkArtist,
      checkAlbum: checkAlbum,
      checkGenre: checkGenre,
      checkYear: checkYear,
      checkTrackNumber: checkTrackNumber,
      checkCover: checkCover,
    );
  }

  // ---------------------------------------------------------------------------
  // Write Track Lyrics
  // ---------------------------------------------------------------------------

  Future<WriteTrackLyricsResponse?> writeTrackLyrics({
    required String trackId,
    required String lyrics,
    required bool synced,
  }) async {
    if (_isRemote) {
      return APIService.instance.writeTrackLyrics(
        trackId: trackId,
        lyrics: lyrics,
        synced: synced,
      );
    }
    return RinfService.instance.writeTrackLyrics(
      trackId: trackId,
      lyrics: lyrics,
      synced: synced,
    );
  }

  // ---------------------------------------------------------------------------
  // Romajize Lyrics
  // ---------------------------------------------------------------------------

  Future<RomajizeLyricsResponse?> romajizeLyrics({
    required String lyrics,
    required bool synced,
    String? lang,
  }) async {
    return _isRemote
        ? APIService.instance.romajizeLyrics(
            lyrics: lyrics,
            synced: synced,
            lang: lang,
          )
        : RinfService.instance.romajizeLyrics(
            lyrics: lyrics,
            synced: synced,
            lang: lang,
          );
  }

  // ---------------------------------------------------------------------------
  // Duplicate Finder
  // ---------------------------------------------------------------------------

  Future<FindDuplicatesResponse?> findDuplicates({
    required bool checkFingerprint,
    required bool checkMbid,
    required bool checkFileSizeDuration,
    required bool checkTitleArtist,
    double? minConfidence,
    String? sourceId,
  }) async {
    if (_isRemote) {
      return APIService.instance.findDuplicates(
        checkFingerprint: checkFingerprint,
        checkMbid: checkMbid,
        checkFileSizeDuration: checkFileSizeDuration,
        checkTitleArtist: checkTitleArtist,
        minConfidence: minConfidence,
        sourceId: sourceId,
      );
    }
    return RinfService.instance.findDuplicates(
      checkFingerprint: checkFingerprint,
      checkMbid: checkMbid,
      checkFileSizeDuration: checkFileSizeDuration,
      checkTitleArtist: checkTitleArtist,
      minConfidence: minConfidence,
      sourceId: sourceId,
    );
  }

  // ---------------------------------------------------------------------------
  // Internal helpers
  // ---------------------------------------------------------------------------

  // ---------------------------------------------------------------------------
  // ListenBrainz: Update Now Playing
  // ---------------------------------------------------------------------------

  Future<bool> updateNowPlaying(String userId, String trackId) async {
    return _isRemote
        ? APIService.instance.updateNowPlaying(trackId)
        : RinfService.instance.updateNowPlaying(userId, trackId);
  }

  // ---------------------------------------------------------------------------
  // ListenBrainz: Get Recommendations
  // ---------------------------------------------------------------------------

  Future<
    ({
      List<DiscoveryRecording> recordings,
      String? playlistTitle,
      String? playlistId,
      int? playlistCount,
    })
  >
  getLBRecommendations({
    required String recType,
    int? count,
    int? offset,
    int? index,
  }) async {
    if (_isRemote) {
      return APIService.instance.getLBRecommendations(
        recType: recType,
        count: count,
        offset: offset,
        index: index,
      );
    }
    final userId = SettingsManager.currentUserId.value ?? '';
    if (userId.isEmpty) {
      return (
        recordings: <DiscoveryRecording>[],
        playlistTitle: null,
        playlistId: null,
        playlistCount: null,
      );
    }
    final response = await RinfService.instance.getLBRecommendations(
      userId,
      recType: recType,
      count: count,
      offset: offset,
      index: index,
    );
    if (response == null) {
      return (
        recordings: <DiscoveryRecording>[],
        playlistTitle: null,
        playlistId: null,
        playlistCount: null,
      );
    }
    return (
      recordings: response.recommendations,
      playlistTitle: response.playlistTitle,
      playlistId: response.playlistId,
      playlistCount: response.playlistCount,
    );
  }

  // ---------------------------------------------------------------------------
  // ListenBrainz: Validate Token
  // ---------------------------------------------------------------------------

  Future<({bool valid, String? userName, String message})> validateLBToken(
    String token,
  ) async {
    if (_isRemote) {
      return APIService.instance.validateLBToken(token);
    }
    final response = await RinfService.instance.validateLBToken(token);
    return (
      valid: response?.valid ?? false,
      userName: response?.userName,
      message: response?.message ?? 'Request failed',
    );
  }

  // ---------------------------------------------------------------------------
  // ListenBrainz: Sync Recommendation Tracks (consolidated)
  // ---------------------------------------------------------------------------

  Future<
    ({
      bool success,
      List<String> addedSources,
      List<String> removedSources,
      int tracksAdded,
      int tracksRemoved,
      String? error,
    })
  >
  syncRecs({required String includedKeys}) async {
    if (_isRemote) {
      return APIService.instance.syncRecs(includedKeys: includedKeys);
    }
    final userId = SettingsManager.currentUserId.value ?? '';
    if (userId.isEmpty) {
      return (
        success: false,
        addedSources: <String>[],
        removedSources: <String>[],
        tracksAdded: 0,
        tracksRemoved: 0,
        error: 'Not logged in' as String?,
      );
    }
    final response = await RinfService.instance.syncRecs(
      userId: userId,
      includedKeys: includedKeys,
    );
    return (
      success: response?.success ?? false,
      addedSources: response != null
          ? List<String>.from(response.addedSources)
          : <String>[],
      removedSources: response != null
          ? List<String>.from(response.removedSources)
          : <String>[],
      tracksAdded: response?.tracksAdded ?? 0,
      tracksRemoved: response?.tracksRemoved ?? 0,
      error: response?.error,
    );
  }

  Future<bool> _localRestartServer() async {
    return false;
  }
}
