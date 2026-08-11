import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/utils/logger.dart';

class RinfService {
  static final RinfService _instance = RinfService._();
  static RinfService get instance => _instance;
  RinfService._();

  // ---------------------------------------------------------------------------
  // Crypt
  // ---------------------------------------------------------------------------

  Future<String?> decrypt(String encryptedKey, {String? masterKey}) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = DecryptResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    DecryptRequest(
      id: id,
      encryptedKey: encryptedKey,
      masterKey: masterKey,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.decryptedKey.isNotEmpty
        ? signal.message.decryptedKey
        : null;
  }

  Future<String?> encrypt(String plainKey, {String? masterKey}) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = EncryptResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    EncryptRequest(
      id: id,
      plainKey: plainKey,
      masterKey: masterKey,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.encryptedKey.isNotEmpty
        ? signal.message.encryptedKey
        : null;
  }

  Future<({String encryptedApiKey, String decryptedApiKey, String masterKey})>
  requestNewApiKey({String? masterKey, required String userId}) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = NewApiKey.rustSignalStream.where((s) => s.message.id == id);
    RequestNewApiKey(
      id: id,
      userId: userId,
      masterKey: masterKey,
    ).sendSignalToRust();
    final signal = await stream.first;
    return (
      encryptedApiKey: signal.message.encryptedApiKey,
      decryptedApiKey: signal.message.decryptedApiKey,
      masterKey: signal.message.masterKey,
    );
  }

  Future<String?> generateMasterKey() async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = GenerateMasterKeyResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    GenerateMasterKeyRequest(id: id).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.masterKey;
  }

  Future<({bool success, String userId, String username})> login(
    String iuser,
    String ipass,
    String ruser,
    String rpass,
  ) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = LoginResult.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    Login(
      id: id,
      iuser: iuser,
      ipass: ipass,
      ruser: ruser,
      rpass: rpass,
    ).sendSignalToRust();
    final signal = await stream.first;
    return (
      success: signal.message.success,
      userId: signal.message.userId,
      username: signal.message.username,
    );
  }

  Future<
    ({
      bool found,
      String userId,
      String username,
      String displayName,
      String role,
    })
  >
  getUserById(String userId) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = GetUserByIdResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    GetUserByIdRequest(id: id, userId: userId).sendSignalToRust();
    final signal = await stream.first;
    return (
      found: signal.message.found,
      userId: signal.message.userId,
      username: signal.message.username,
      displayName: signal.message.displayName,
      role: signal.message.role,
    );
  }

  Future<
    ({
      bool found,
      String userId,
      String username,
      String displayName,
      String role,
    })
  >
  getUserByUsername(String username) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = GetUserByUsernameResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    GetUserByUsernameRequest(id: id, username: username).sendSignalToRust();
    final signal = await stream.first;
    return (
      found: signal.message.found,
      userId: signal.message.userId,
      username: signal.message.username,
      displayName: signal.message.displayName,
      role: signal.message.role,
    );
  }

  // ---------------------------------------------------------------------------
  // Library
  // ---------------------------------------------------------------------------

  Future<List<TrackInfo>> getTracks({String? albumId}) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = ListTracksResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    ListTracksRequest(id: id, albumId: albumId).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.tracks;
  }

  Future<List<AlbumInfo>> getAlbums({String? artistId}) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = ListAlbumsResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    ListAlbumsRequest(id: id, artistId: artistId).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.albums;
  }

  Future<List<ArtistInfo>> getArtists() async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = ListArtistsResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    ListArtistsRequest(id: id).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.artists;
  }

  Future<List<PlaylistInfo>> getPlaylists() async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = ListPlaylistsResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    ListPlaylistsRequest(id: id).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.playlists;
  }

  Future<String> createPlaylist(String userId, String name) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = CreatePlaylistResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    CreatePlaylistRequest(
      id: id,
      userId: userId,
      name: name,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.playlistId;
  }

  Future<bool> deletePlaylist(String playlistId) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = DeletePlaylistResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    DeletePlaylistRequest(id: id, playlistId: playlistId).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.success;
  }

  // ---------------------------------------------------------------------------
  // Playlist tracks
  // ---------------------------------------------------------------------------

  Future<List<TrackInfo>> getPlaylistTracks(String playlistId) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = GetPlaylistTracksResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    GetPlaylistTracksRequest(id: id, playlistId: playlistId).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.tracks;
  }

  Future<bool> addTrackToPlaylist(String playlistId, String trackId) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = AddTrackToPlaylistResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    AddTrackToPlaylistRequest(
      id: id,
      playlistId: playlistId,
      trackId: trackId,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.success;
  }

  Future<bool> removeTrackFromPlaylist(
    String playlistId,
    String trackId,
  ) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = RemoveTrackFromPlaylistResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    RemoveTrackFromPlaylistRequest(
      id: id,
      playlistId: playlistId,
      trackId: trackId,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.success;
  }

  Future<bool> reorderPlaylistTracks(
    String playlistId,
    List<String> trackIds,
  ) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = ReorderPlaylistTracksResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    ReorderPlaylistTracksRequest(
      id: id,
      playlistId: playlistId,
      trackIds: trackIds,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.success;
  }

  // ---------------------------------------------------------------------------
  // Playback
  // ---------------------------------------------------------------------------

  Future<({String filePath, String? error, List<List<String>>? headers})>
  playTrack(String? trackId, {TrackInfo? track}) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = PlayTrackResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    PlayTrackRequest(id: id, trackId: trackId, track: track).sendSignalToRust();
    final signal = await stream.first;
    final headers = signal.message.headers
        ?.map((h) => [h.item1, h.item2])
        .toList();
    return (
      filePath: signal.message.filePath,
      error: signal.message.error,
      headers: headers,
    );
  }

  Future<PreviewTrackResponse> previewTrack(TrackInfo track) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = PreviewTrackResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    PreviewTrackRequest(id: id, track: track).sendSignalToRust();
    final signal = await stream.first;
    if (signal.message.error != null) {
      log(
        "Error fetching preview for track ${track.title}: ${signal.message.error}",
        isError: true,
      );
    }
    return signal.message;
  }

  Future<TrackInfo?> getTrackInfo(String trackId) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = GetTrackResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    GetTrackRequest(id: id, trackId: trackId).sendSignalToRust();
    final signal = await stream.first;
    if (signal.message.error != null) return null;
    return signal.message.track;
  }

  Future<bool> reportPlayback(
    String userId,
    String trackId,
    String playedAt,
    String source,
  ) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = ReportPlaybackResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    ReportPlaybackRequest(
      id: id,
      userId: userId,
      trackId: trackId,
      playedAt: playedAt,
      source: source,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.success;
  }

  Future<List<PlaybackRecord>> getHistory(
    String userId, {
    int limit = 50,
  }) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = GetHistoryResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    GetHistoryRequest(id: id, userId: userId, limit: limit).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.records;
  }

  // ---------------------------------------------------------------------------
  // Downloads (unified)
  // ---------------------------------------------------------------------------

  Future<List<DownloadRecord>> listDownloads(
    String userId, {
    String? source,
  }) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = ListDownloadsResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    ListDownloadsRequest(
      id: id,
      userId: userId,
      source: source,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.downloads;
  }

  Future<({List<DlSearchItem> results, bool success, String? error})> search(
    String sourceType,
    String query,
  ) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = DownloadSearchResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    DownloadSearchRequest(
      id: id,
      sourceType: sourceType,
      query: query,
    ).sendSignalToRust();
    final signal = await stream.first;
    return (
      results: signal.message.results,
      success: signal.message.success,
      error: signal.message.error,
    );
  }

  Future<({bool success, String downloadId, String? error})> create(
    String sourceType,
    String url,
    String dest,
    String userId, {
    String? extra,
  }) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = DownloadCreateResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    DownloadCreateRequest(
      id: id,
      sourceType: sourceType,
      url: url,
      dest: dest,
      userId: userId,
      extra: extra,
    ).sendSignalToRust();
    final signal = await stream.first;
    return (
      success: signal.message.success,
      downloadId: signal.message.downloadId,
      error: signal.message.error,
    );
  }

  Future<({bool success, String? error})> cancel(
    String sourceType,
    String downloadId, {
    String? extra,
  }) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = DownloadCancelResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    DownloadCancelRequest(
      id: id,
      sourceType: sourceType,
      downloadId: downloadId,
      extra: extra,
    ).sendSignalToRust();
    final signal = await stream.first;
    return (success: signal.message.success, error: signal.message.error);
  }

  Future<({bool success, String? version, String? error})> testConnection(
    String sourceType, {
    String? url,
    String? token,
    String? username,
    String? password,
  }) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = DownloadTestConnectionResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    DownloadTestConnectionRequest(
      id: id,
      sourceType: sourceType,
      url: url,
      token: token,
      username: username,
      password: password,
    ).sendSignalToRust();
    final signal = await stream.first;
    return (
      success: signal.message.success,
      version: signal.message.version,
      error: signal.message.error,
    );
  }

  Future<String?> getCurrentVersion(String app) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = GetCurrentVersionResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    GetCurrentVersionRequest(id: id, app: app).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.version;
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
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = GetLatestVersionResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    GetLatestVersionRequest(
      id: id,
      owner: owner,
      repo: repo,
      nightly: nightly,
      atomic: atomic,
    ).sendSignalToRust();
    final signal = await stream.first;
    return (
      version: signal.message.version,
      tagName: signal.message.tagName,
      releaseNotes: signal.message.releaseNotes,
      publishedAt: signal.message.publishedAt,
      error: signal.message.error,
    );
  }

  Future<({bool success, String info, String? error})> getInfo(
    String sourceType,
    String url,
  ) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = DownloadGetInfoResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    DownloadGetInfoRequest(
      id: id,
      sourceType: sourceType,
      url: url,
    ).sendSignalToRust();
    final signal = await stream.first;
    return (
      success: signal.message.success,
      info: signal.message.info,
      error: signal.message.error,
    );
  }

  Future<List<DownloadRecord>> pollDownloads(String userId) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = DownloadsPollResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    DownloadsPollRequest(id: id, userId: userId).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.downloads;
  }

  // ---------------------------------------------------------------------------
  // Settings
  // ---------------------------------------------------------------------------

  Future<bool> setUserSetting(String userId, String key, String value) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = SetUserSettingResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    SetUserSettingRequest(
      id: id,
      userId: userId,
      key: key,
      value: value,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.success;
  }

  Future<String> getUserSetting(String userId, String key) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = GetUserSettingResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    GetUserSettingRequest(id: id, userId: userId, key: key).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.value;
  }

  Future<Map<String, String>> getAllUserSettings(String userId) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = GetAllUserSettingsResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    GetAllUserSettingsRequest(id: id, userId: userId).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.settings;
  }

  // ---------------------------------------------------------------------------
  // Cover
  // ---------------------------------------------------------------------------

  Future<Uint8List?> getAlbumCover(String albumId) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = GetAlbumCoverResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    GetAlbumCoverRequest(id: id, albumId: albumId).sendSignalToRust();
    final signal = await stream.first;
    final cover = signal.message.cover;
    return cover != null ? Uint8List.fromList(cover) : null;
  }

  Future<Uint8List?> getTrackCover(String trackId) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = GetTrackCoverResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    GetTrackCoverRequest(id: id, trackId: trackId).sendSignalToRust();
    final signal = await stream.first;
    final cover = signal.message.cover;
    return cover != null ? Uint8List.fromList(cover) : null;
  }

  // ---------------------------------------------------------------------------
  // Scan
  // ---------------------------------------------------------------------------

  Future<({bool started, String? error})> scanLibrary({
    required String userId,
    required bool force,
  }) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = ScanLibraryResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    ScanLibraryRequest(id: id, userId: userId, force: force).sendSignalToRust();
    final result = await stream.first;
    return (started: result.message.started, error: result.message.error);
  }

  Future<({bool running, ScanProgressSignal? progress})> getScanStatus() async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = ScanStatusResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    ScanStatusRequest(id: id).sendSignalToRust();
    final result = await stream.first;
    return (running: result.message.running, progress: result.message.progress);
  }

  Future<({bool started, String? error})> scanSource({
    required String userId,
    required String sourceId,
    required bool force,
  }) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = ScanSourceResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    ScanSourceRequest(
      id: id,
      userId: userId,
      sourceId: sourceId,
      force: force,
    ).sendSignalToRust();
    final result = await stream.first;
    return (started: result.message.started, error: result.message.error);
  }

  Future<void> startPeriodicScan() async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = StartPeriodicScanResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    StartPeriodicScanRequest(id: id).sendSignalToRust();
    await stream.first;
  }

  // ---------------------------------------------------------------------------
  // Library Sources
  // ---------------------------------------------------------------------------

  Future<List<LibrarySourceInfo>> listLibrarySources(String userId) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = ListLibrarySourcesResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    ListLibrarySourcesRequest(id: id, userId: userId).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.sources;
  }

  Future<List<LibrarySourceInfo>> listEditableSources(String userId) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = ListEditableSourcesResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    ListEditableSourcesRequest(id: id, userId: userId).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.sources;
  }

  Future<({String sourceId, bool success})> addLibrarySource(
    String userId,
    String url,
    String name,
    String sourceType,
  ) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = AddLibrarySourceResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    AddLibrarySourceRequest(
      id: id,
      userId: userId,
      url: url,
      name: name,
      sourceType: sourceType,
    ).sendSignalToRust();
    final signal = await stream.first;
    return (sourceId: signal.message.sourceId, success: signal.message.success);
  }

  Future<bool> removeLibrarySource(String userId, String sourceId) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = RemoveLibrarySourceResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    RemoveLibrarySourceRequest(
      id: id,
      userId: userId,
      sourceId: sourceId,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.success;
  }

  Future<List<JellyfinLibraryInfo>> testJellyfinSource(String url) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = TestJellyfinSourceResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    TestJellyfinSourceRequest(id: id, url: url).sendSignalToRust();
    final signal = await stream.first;
    if (signal.message.error != null && signal.message.error!.isNotEmpty) {
      throw Exception(signal.message.error);
    }
    return signal.message.libraries;
  }

  // ---------------------------------------------------------------------------
  // Identify
  // ---------------------------------------------------------------------------

  Future<List<TrackInfo>> listUnidentifiedTracks({String? sourceId}) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = ListUnidentifiedTracksResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    ListUnidentifiedTracksRequest(
      id: id,
      sourceId: sourceId,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.tracks;
  }

  Future<List<TrackInfo>> listDownloadFolderTracks(String path) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = ListDownloadFolderTracksResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    ListDownloadFolderTracksRequest(id: id, path: path).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.tracks;
  }

  Future<List<TrackInfo>> listTracksBySource(String sourceId) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = ListTracksBySourceResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    ListTracksBySourceRequest(id: id, sourceId: sourceId).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.tracks;
  }

  Future<String?> getAlbumMbid(String albumId) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = GetAlbumMbidResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    GetAlbumMbidRequest(id: id, albumId: albumId).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.mbid;
  }

  Future<List<MatchCandidate>> identifySingleTrack(String trackId) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = IdentifySingleTrackResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    IdentifySingleTrackRequest(id: id, trackId: trackId).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.candidates;
  }

  Future<List<RecordingInfo>> searchMusicBrainz(String query) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = EnhancedSearchResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    EnhancedSearchRequest(id: id, query: query).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.recordings;
  }

  Future<GetReleaseTracksResponse> getReleaseTracks(String releaseId) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = GetReleaseTracksResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    GetReleaseTracksRequest(id: id, releaseId: releaseId).sendSignalToRust();
    final signal = await stream.first;
    return signal.message;
  }

  Future<RecordingInfo?> fetchRecording(String mbid) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = FetchRecordingResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    FetchRecordingRequest(id: id, mbid: mbid).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.recording;
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
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = ApplyIdentificationResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    ApplyIdentificationRequest(
      id: id,
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
    ).sendSignalToRust();
    final signal = await stream.first;
    return (
      success: signal.message.success,
      error: signal.message.error,
      newFilePath: signal.message.newFilePath,
    );
  }

  // ---------------------------------------------------------------------------
  // Tag editor (standalone file tag reader/writer)
  // ---------------------------------------------------------------------------

  Future<ReadFileTagsResponse> readFileTags(String path) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = ReadFileTagsResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    ReadFileTagsRequest(id: id, path: path).sendSignalToRust();
    return (await stream.first).message;
  }

  Future<({bool success, String? error})> writeFileTags({
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
  }) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = WriteFileTagsResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    WriteFileTagsRequest(
      id: id,
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
    ).sendSignalToRust();
    final signal = await stream.first;
    return (success: signal.message.success, error: signal.message.error);
  }

  Future<ReadFileTagsResponse> readFileTagsBytes(
    String filename,
    Uint8List bytes,
  ) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = ReadFileTagsResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    ReadFileTagsBytesRequest(
      id: id,
      filename: filename,
      bytes: bytes,
    ).sendSignalToRust();
    return (await stream.first).message;
  }

  Future<({bool success, Uint8List? bytes, String? error})> writeFileTagsBytes({
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
  }) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = WriteFileTagsBytesResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    WriteFileTagsBytesRequest(
      id: id,
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
    ).sendSignalToRust();
    final signal = await stream.first;
    final result = signal.message;
    return (
      success: result.success,
      bytes: result.success ? Uint8List.fromList(result.bytes) : null,
      error: result.error,
    );
  }

  // ---------------------------------------------------------------------------
  // Tagging
  // ---------------------------------------------------------------------------

  Future<String> createTagging(String name, String description) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = CreateTaggingResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    CreateTaggingRequest(
      id: id,
      name: name,
      description: description,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.success;
  }

  Future<String> putTagging(String name, String tag) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = PutTaggingResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    PutTaggingRequest(id: id, name: name, tag: tag).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.success;
  }

  // ---------------------------------------------------------------------------
  // Account
  // ---------------------------------------------------------------------------

  Future<List<UserListItem>> listUsers() async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = ListUsersResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    ListUsersRequest(id: id).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.users;
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
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = UpdateAccountResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    UpdateAccountRequest(
      id: id,
      operatorUserId: operatorUserId,
      operatorPassword: operatorPassword,
      targetUserId: targetUserId,
      newUsername: newUsername,
      newPassword: newPassword,
      displayName: newDisplayName,
      role: role,
    ).sendSignalToRust();
    final signal = await stream.first;
    return (
      success: signal.message.success,
      userId: signal.message.userId,
      username: signal.message.username,
      displayName: signal.message.displayName,
      role: signal.message.role,
      apiKey: signal.message.apiKey,
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
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = CreateAccountResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    CreateAccountRequest(
      id: id,
      adminUsername: adminUsername,
      adminPassword: adminPassword,
      username: username,
      password: password,
      displayName: displayName,
      role: role,
    ).sendSignalToRust();
    final signal = await stream.first;
    return (
      success: signal.message.success,
      userId: signal.message.userId,
      username: signal.message.username,
      displayName: signal.message.displayName,
      role: signal.message.role,
      apiKey: signal.message.apiKey,
    );
  }

  Future<({bool success, String username})> deleteUser({
    required String adminUsername,
    required String adminPassword,
    required String targetUsername,
  }) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = DeleteAccountResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    DeleteAccountRequest(
      id: id,
      adminUsername: adminUsername,
      adminPassword: adminPassword,
      targetUsername: targetUsername,
    ).sendSignalToRust();
    final signal = await stream.first;
    return (success: signal.message.success, username: signal.message.username);
  }

  Future<bool> verifyCurrentPassword(String userId, String password) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = VerifyCurrentPasswordResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    VerifyCurrentPasswordRequest(
      id: id,
      userId: userId,
      password: password,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.valid;
  }

  // ---------------------------------------------------------------------------
  // System
  // ---------------------------------------------------------------------------

  Future<bool> initDatabase(String path, {required String masterKey}) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = InitDatabaseResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    InitDatabase(id: id, path: path, masterKey: masterKey).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.success;
  }

  void startServer({
    required int port,
    required String masterKey,
    required String configPath,
  }) {
    StartServer(
      port: port,
      masterKey: masterKey,
      configPath: configPath,
    ).sendSignalToRust();
  }

  Future<
    ({
      bool success,
      String? error,
      Map<String, dynamic>? settings,
      bool isFirstRun,
    })
  >
  initConfig(String path) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = InitConfigResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    InitConfig(id: id, path: path).sendSignalToRust();
    final signal = await stream.first;
    Map<String, dynamic>? settings;
    final json = signal.message.settingsJson;
    if (json != null && json.isNotEmpty) {
      try {
        settings = jsonDecode(json) as Map<String, dynamic>;
      } catch (_) {}
    }
    return (
      success: signal.message.success,
      error: signal.message.error,
      settings: settings,
      isFirstRun: signal.message.isFirstRun,
    );
  }

  Future<({bool success, String? error})> saveConfig(
    Map<String, dynamic> settings,
  ) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = SaveConfigResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    SaveConfigRequest(
      id: id,
      settingsJson: jsonEncode(settings),
    ).sendSignalToRust();
    final signal = await stream.first;
    return (success: signal.message.success, error: signal.message.error);
  }

  Stream<LogSignal> get logSignal =>
      LogSignal.rustSignalStream.map((s) => s.message);

  // ---------------------------------------------------------------------------
  // Fingerprint track
  // ---------------------------------------------------------------------------

  Future<RecordingInfo?> fingerprintTrack(
    String trackId, {
    String? filePath,
  }) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = FingerprintTrackResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    FingerprintTrackRequest(
      id: id,
      trackId: trackId,
      filePath: filePath,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.recording;
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
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = FetchLyricsResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    FetchLyricsRequest(
      id: id,
      title: title,
      artist: artist,
      album: album,
      duration: duration,
      preferSync: preferSync,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.result;
  }

  Future<List<LyricsResult>> searchLyrics({
    required String query,
    bool preferSync = true,
  }) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = SearchLyricsResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    SearchLyricsRequest(
      id: id,
      query: query,
      preferSync: preferSync,
    ).sendSignalToRust();
    final signal = await stream.first;
    if (signal.message.error != null) {
      log('searchLyrics error: ${signal.message.error}', isError: true);
    }
    return signal.message.results;
  }

  // ---------------------------------------------------------------------------
  // Tools
  // ---------------------------------------------------------------------------

  Future<GetLibraryStatsResponse?> getLibraryStats({
    String? namingPattern,
  }) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = GetLibraryStatsResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    GetLibraryStatsRequest(
      id: id,
      namingPattern: namingPattern,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message;
  }

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
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = FormatNamingPreviewResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    FormatNamingPreviewRequest(
      id: id,
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
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.result;
  }

  // ---------------------------------------------------------------------------
  // Rename
  // ---------------------------------------------------------------------------

  Future<BatchRenamePreviewResponse?> batchRenamePreview({
    required List<String> filePaths,
    required String pattern,
    String? sourceId,
  }) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = BatchRenamePreviewResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    BatchRenamePreviewRequest(
      id: id,
      filePaths: filePaths,
      pattern: pattern,
      sourceId: sourceId,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message;
  }

  Future<BatchRenameApplyResponse?> batchRenameApply({
    required List<String> filePaths,
    required List<String> trackIds,
    required String pattern,
  }) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = BatchRenameApplyResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    BatchRenameApplyRequest(
      id: id,
      filePaths: filePaths,
      trackIds: trackIds,
      pattern: pattern,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message;
  }

  Future<CheckNamingConventionResponse?> checkNamingConvention({
    String? sourceId,
    required String pattern,
  }) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = CheckNamingConventionResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    CheckNamingConventionRequest(
      id: id,
      sourceId: sourceId,
      pattern: pattern,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message;
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
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = FindMissingMetadataResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    FindMissingMetadataRequest(
      id: id,
      checkTitle: checkTitle,
      checkArtist: checkArtist,
      checkAlbum: checkAlbum,
      checkGenre: checkGenre,
      checkYear: checkYear,
      checkTrackNumber: checkTrackNumber,
      checkCover: checkCover,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message;
  }

  // ---------------------------------------------------------------------------
  // Write Track Lyrics
  // ---------------------------------------------------------------------------

  Future<WriteTrackLyricsResponse?> writeTrackLyrics({
    required String trackId,
    required String lyrics,
    required bool synced,
  }) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = WriteTrackLyricsResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    WriteTrackLyricsRequest(
      id: id,
      trackId: trackId,
      lyrics: lyrics,
      synced: synced,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message;
  }

  // ---------------------------------------------------------------------------
  // Romajize Lyrics
  // ---------------------------------------------------------------------------

  Future<RomajizeLyricsResponse?> romajizeLyrics({
    required String lyrics,
    required bool synced,
    String? lang,
  }) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = RomajizeLyricsResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    RomajizeLyricsRequest(
      id: id,
      lyrics: lyrics,
      synced: synced,
      lang: lang,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message;
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
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = FindDuplicatesResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    FindDuplicatesRequest(
      id: id,
      checkFingerprint: checkFingerprint,
      checkMbid: checkMbid,
      checkFileSizeDuration: checkFileSizeDuration,
      checkTitleArtist: checkTitleArtist,
      minConfidence: minConfidence,
      sourceId: sourceId,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message;
  }

  // ---------------------------------------------------------------------------
  // ListenBrainz: Update Now Playing
  // ---------------------------------------------------------------------------

  Future<bool> updateNowPlaying(String userId, String trackId) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = UpdateNowPlayingResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    UpdateNowPlayingRequest(
      id: id,
      userId: userId,
      trackId: trackId,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message.success;
  }

  // ---------------------------------------------------------------------------
  // ListenBrainz: Get Recommendations
  // ---------------------------------------------------------------------------

  Future<GetLBRecommendationsResponse?> getLBRecommendations(
    String userId, {
    required String recType,
    int? count,
    int? offset,
    int? index,
  }) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = GetLBRecommendationsResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    GetLBRecommendationsRequest(
      id: id,
      userId: userId,
      recType: recType,
      count: count,
      offset: offset,
      index: index,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message;
  }

  // ---------------------------------------------------------------------------
  // ListenBrainz: Validate Token
  // ---------------------------------------------------------------------------

  Future<ValidateLBTokenResponse?> validateLBToken(String token) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = ValidateLBTokenResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    ValidateLBTokenRequest(id: id, token: token).sendSignalToRust();
    final signal = await stream.first;
    return signal.message;
  }

  // ---------------------------------------------------------------------------
  // ListenBrainz: Sync Recommendation Tracks (consolidated)
  // ---------------------------------------------------------------------------

  Future<SyncRecsResponse?> syncRecs({
    required String userId,
    required String includedKeys,
  }) async {
    final id = DateTime.now().microsecondsSinceEpoch.toString();
    final stream = SyncRecsResponse.rustSignalStream.where(
      (s) => s.message.id == id,
    );
    SyncRecsRequest(
      id: id,
      userId: userId,
      includedKeys: includedKeys,
    ).sendSignalToRust();
    final signal = await stream.first;
    return signal.message;
  }
}
