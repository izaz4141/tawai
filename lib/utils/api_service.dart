import 'dart:async';
import 'dart:convert';
import 'package:http/http.dart' as http;
import 'package:flutter/foundation.dart';

import 'package:tawai/src/bindings/bindings.dart';

import 'package:tawai/utils/io_service.dart';
import 'package:tawai/utils/platform_service.dart';

import 'package:tawai/utils/bindings_json.dart';
import 'package:tawai/utils/settings.dart';
import 'package:tawai/utils/logger.dart';
import 'package:tawai/utils/rinf_service.dart';
import 'package:tawai/utils/system_service.dart';
import 'package:tawai/models/fs.dart';
import 'package:tawai/models/user.dart';

class APIService {
  static final APIService _instance = APIService._();
  static APIService get instance => _instance;
  APIService._();

  static final ValueNotifier<bool> isOnline = ValueNotifier(false);
  static final ValueNotifier<String?> serverVersion = ValueNotifier(null);
  static Timer? _timer;
  static Timer? _debounce;

  String _jwt = '';
  String _csrfToken = '';

  bool get isAuthenticated => _jwt.isNotEmpty;

  Map<String, String> _authHeaders({Map<String, String>? extra}) {
    final headers = <String, String>{};
    if (kIsWeb) {
      final csrf = IOServiceFactory.create().getCookie('tawai_csrf');
      if (csrf != null && csrf.isNotEmpty) {
        headers['X-CSRF-TOKEN'] = csrf;
      }
    } else {
      final apiKey = SettingsManager.currentUser.value?.apiKey ?? '';
      if (PlatformService().isRemote && apiKey.isNotEmpty) {
        headers['X-API-Key'] = apiKey;
      } else {
        if (_jwt.isNotEmpty) {
          headers['Cookie'] = 'tawai_jwt=$_jwt';
        }
        if (_csrfToken.isNotEmpty) {
          headers['X-CSRF-TOKEN'] = _csrfToken;
        }
      }
    }
    if (extra != null) headers.addAll(extra);
    return headers;
  }

  String _newId() => DateTime.now().microsecondsSinceEpoch.toString();

  Future<void> init() async {
    if (kIsWeb) {
      final success = await login(username: '', password: '');
      if (success) {
        SettingsManager.isLoggedIn.value = true;
      }
    }

    _startPolling();

    SettingsManager.serverHost.addListener(restartPolling);
    SettingsManager.serverPort.addListener(restartPolling);
  }

  String get baseUrl {
    if (kIsWeb) {
      return Uri.base.origin;
    }
    String host = SettingsManager.serverHost.value;
    if (!host.contains('://')) {
      host = 'http://$host';
    }
    final port = SettingsManager.serverPort.value;
    return '$host:$port';
  }

  String wrapImageUrl(String externalUrl) {
    if (externalUrl.isEmpty) return externalUrl;
    if (!kIsWeb) return externalUrl;
    final encoded = Uri.encodeComponent(externalUrl);
    return '$baseUrl/api/tawai/utils/img?url=$encoded';
  }

  Future<Uint8List?> getAlbumCover(String albumId) async {
    try {
      final response = await http.get(
        Uri.parse('$baseUrl/api/tawai/library/albums/$albumId/cover'),
        headers: _authHeaders(),
      );
      if (response.statusCode == 200 && response.bodyBytes.isNotEmpty) {
        return response.bodyBytes;
      }
    } catch (e) {
      log('getAlbumCover error: $e', isError: true);
    }
    return null;
  }

  Future<Uint8List?> getTrackCover(String trackId) async {
    try {
      final response = await http.get(
        Uri.parse('$baseUrl/api/tawai/library/tracks/$trackId/cover'),
        headers: _authHeaders(),
      );
      if (response.statusCode == 200 && response.bodyBytes.isNotEmpty) {
        return response.bodyBytes;
      }
    } catch (e) {
      log('getTrackCover error: $e', isError: true);
    }
    return null;
  }

  void _startPolling() {
    _timer?.cancel();

    isOnline.value = false;
    serverVersion.value = null;

    _checkStatus();
    _timer = Timer.periodic(const Duration(seconds: 5), (_) => _checkStatus());
  }

  void restartPolling() {
    _debounce?.cancel();
    _debounce = Timer(const Duration(milliseconds: 200), () {
      _startPolling();
    });
  }

  Future<bool> login({
    required String username,
    required String password,
  }) async {
    try {
      final credentials = base64Encode(utf8.encode('$username:$password'));

      Map<String, String> requestHeaders = {
        'Authorization': 'Basic $credentials',
      };

      if (kIsWeb) {
        final String? csrfToken = IOServiceFactory.create().getCookie(
          'tawai_csrf',
        );
        if (csrfToken != null && csrfToken.isNotEmpty) {
          requestHeaders['X-CSRF-TOKEN'] = csrfToken;
        }
      }

      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/auth/login'),
        headers: requestHeaders,
      );

      if (response.statusCode == 200) {
        final data = jsonDecode(response.body);
        _jwt = data['access_token'] as String? ?? '';
        _csrfToken = data['csrf_token'] as String? ?? '';

        if (_jwt.isNotEmpty) {
          final userData = data['user'] as Map<String, dynamic>?;
          if (userData != null) {
            SettingsManager.currentUser.value = User.fromJson(userData);
            SettingsManager.currentUserId.value = userData['id'] ?? '';
          }
          if (!kIsWeb) {
            await RinfService.instance.saveConfig({
              'current_user': SettingsManager.currentUserId.value ?? '',
            });
          }
          await SettingsManager.loadFromBackend();
          await SettingsManager.loadAllUserSettings();
          SettingsManager.attachAutoSave();
          return true;
        }
      } else {
        log(
          'Login failed: ${response.statusCode} ${response.body}',
          isError: true,
        );
      }
    } catch (e, stack) {
      log('Login error: $e \n$stack', isError: true);
    }

    return false;
  }

  Future<
    ({
      bool success,
      String id,
      String apiKey,
      String username,
      String displayName,
      String role,
    })
  >
  testLogin({
    required String host,
    required int port,
    required String username,
    required String password,
  }) async {
    try {
      final credentials = base64Encode(utf8.encode('$username:$password'));
      final response = await http.post(
        Uri.parse(
          '${host.contains('://') ? host : 'http://$host'}:$port/api/tawai/auth/login',
        ),
        headers: {'Authorization': 'Basic $credentials'},
      );

      if (response.statusCode == 200) {
        final data = jsonDecode(response.body);
        final user = data['user'] as Map<String, dynamic>?;
        if (data['access_token'] is String &&
            (data['access_token'] as String).isNotEmpty &&
            user != null) {
          return (
            success: true,
            id: user['id'] as String? ?? '',
            apiKey: user['api_key'] as String? ?? '',
            username: user['username'] as String? ?? username,
            displayName: user['display_name'] as String? ?? '',
            role: user['role'] as String? ?? 'user',
          );
        }
      }
    } catch (e) {
      log('Test login error: $e', isError: true);
    }
    return (
      success: false,
      id: '',
      apiKey: '',
      username: username,
      displayName: '',
      role: 'user',
    );
  }

  Future<bool> regenerateApiKey() async {
    try {
      final response = await http.get(
        Uri.parse('$baseUrl/api/tawai/auth/generate-api'),
        headers: _authHeaders(),
      );
      if (response.statusCode == 200) {
        final data = jsonDecode(response.body);
        _jwt = data['access_token'] as String? ?? '';
        _csrfToken = data['csrf_token'] as String? ?? '';
        final newKey = data['api_key'] as String? ?? '';
        final user = SettingsManager.currentUser.value;
        if (user != null && newKey.isNotEmpty) {
          SettingsManager.currentUser.value = User(
            id: user.id,
            username: user.username,
            displayName: user.displayName,
            passwordHash: user.passwordHash,
            apiKey: newKey,
            role: user.role,
            createdAt: user.createdAt,
            updatedAt: user.updatedAt,
          );
        }
        await SettingsManager.loadFromBackend();
        return true;
      }
      log(
        'Regen API-Key failed: ${response.statusCode} ${response.body}',
        isError: true,
      );
      return false;
    } catch (e) {
      log("Regen API-Key failed: $e", isError: true);
      return false;
    }
  }

  Future<FsListing?> listDirectory(String path) async {
    try {
      final uri = Uri.parse(
        '$baseUrl/api/tawai/system/fs/list',
      ).replace(queryParameters: {'path': path});
      final response = await http.get(uri, headers: _authHeaders());
      if (response.statusCode == 200) {
        return FsListing.fromJson(
          jsonDecode(response.body) as Map<String, dynamic>,
        );
      }
      log(
        "listDirectory failed: ${response.statusCode} ${response.body}",
        isError: true,
      );
    } catch (e) {
      log('listDirectory error: $e', isError: true);
    }
    return null;
  }

  Future<bool> restartServer() async {
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/system/restart'),
        headers: _authHeaders(),
      );
      if (response.statusCode == 200) {
        return true;
      }
      log(
        'Server restart failed: ${response.statusCode} ${response.body}',
        isError: true,
      );
      return false;
    } catch (e) {
      log("Server restart failed: $e", isError: true);
      return false;
    }
  }

  Future<void> _checkStatus() async {
    try {
      final response = await http.get(
        Uri.parse('$baseUrl/api/tawai/system/status'),
      );
      if (response.statusCode == 200) {
        isOnline.value = true;
        try {
          final data = jsonDecode(response.body);
          if (data is Map) {
            serverVersion.value = data['version'] as String?;
          }
        } catch (_) {}
      } else {
        isOnline.value = false;
        serverVersion.value = null;
        log(
          "Server status check failed: ${response.statusCode} ${response.body}",
        );
      }
    } catch (e) {
      if (!isOnline.value) {
      } else {
        log("Server status check failed: $e", isError: true);
      }
      isOnline.value = false;
      serverVersion.value = null;
    }
  }

  Future<String?> getServerVersion() async {
    await _checkStatus();
    return serverVersion.value;
  }

  Future<Map<String, dynamic>?> getSettings() async {
    try {
      final response = await http.get(
        Uri.parse('$baseUrl/api/tawai/settings/global'),
        headers: _authHeaders(),
      );
      if (response.statusCode == 200) {
        return jsonDecode(response.body) as Map<String, dynamic>;
      }
    } catch (e) {
      log("Error getting settings: $e", isError: true);
    }
    return null;
  }

  Future<bool> saveSettings(Map<String, dynamic> settings) async {
    try {
      final response = await http.put(
        Uri.parse('$baseUrl/api/tawai/settings/global'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode(settings),
      );
      return response.statusCode == 200;
    } catch (e) {
      log("Error saving settings: $e", isError: true);
      return false;
    }
  }

  Future<Map<String, String>> getAllUserSettings() async {
    try {
      final response = await http.get(
        Uri.parse('$baseUrl/api/tawai/settings/user'),
        headers: _authHeaders(),
      );
      if (response.statusCode == 200) {
        final json = jsonDecode(response.body) as Map<String, dynamic>;
        final settings = json['settings'] as Map<String, dynamic>;
        return settings.map((k, v) => MapEntry(k, v.toString()));
      }
    } catch (e) {
      log("Error getting all user settings: $e", isError: true);
    }
    return {};
  }

  Future<String?> generateMasterKey() async {
    try {
      final response = await http.get(
        Uri.parse('$baseUrl/api/tawai/auth/generate-master-key'),
        headers: _authHeaders(),
      );
      if (response.statusCode == 200) return response.body;
    } catch (e) {
      log("Error generating master key: $e", isError: true);
    }
    return null;
  }

  Future<String?> getCurrentVersion(String app) async {
    try {
      final response = await http.get(
        Uri.parse('$baseUrl/api/tawai/version/current?app=$app'),
        headers: _authHeaders(),
      );
      if (response.statusCode == 200) {
        final data = jsonDecode(response.body);
        return data['version'] as String?;
      }
    } catch (e) {
      log("Error getting current version: $e", isError: true);
    }
    return null;
  }

  Future<VersionInfo?> getLatestVersion(
    String owner,
    String repo, {
    bool nightly = false,
    bool atomic = true,
  }) async {
    try {
      final response = await http.get(
        Uri.parse(
          '$baseUrl/api/tawai/version/latest?owner=$owner&repo=$repo&nightly=$nightly&atomic=$atomic',
        ),
        headers: _authHeaders(),
      );
      if (response.statusCode == 200) {
        final data = jsonDecode(response.body);

        if (data['error'] != Null) {
          final version = VersionInfo.fromJson(data);
          return version;
        }
      }
    } catch (e) {
      log("Error getting latest version: $e", isError: true);
    }
    return null;
  }

  Future<String?> compareVersions(List<String> versions) async {
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/version/compare'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({'id': _newId(), 'versions': versions}),
      );
      if (response.statusCode == 200) {
        final data = jsonDecode(response.body);
        return data['latest'] as String?;
      }
    } catch (e) {
      log("Error comparing versions: $e", isError: true);
    }
    return null;
  }

  // ---------------------------------------------------------------------------
  // Playback / History
  // ---------------------------------------------------------------------------

  Future<({String filePath, String? error, List<List<String>>? headers})>
  playTrack(String? trackId, {TrackInfo? track}) async {
    try {
      final body = {
        'id': _newId(),
        'track_id': trackId,
        if (track != null) 'track': track.toJson(),
      };
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/playback/play'),
        headers: {..._authHeaders(), 'Content-Type': 'application/json'},
        body: jsonEncode(body),
      );
      if (response.statusCode == 200) {
        final json = jsonDecode(response.body) as Map<String, dynamic>;
        var filePath = json['file_path'] as String? ?? '';
        if (filePath.startsWith('/')) {
          filePath = '$baseUrl$filePath';
        }
        final rawHeaders = json['headers'] as List<dynamic>?;
        final headers = rawHeaders
            ?.map((h) => [(h as List<dynamic>)[0] as String, h[1] as String])
            .toList();
        return (filePath: filePath, error: null, headers: headers);
      } else if (response.statusCode == 404) {
        return (filePath: '', error: 'Track not found', headers: null);
      }
    } catch (e) {
      log('playTrack error: $e', isError: true);
    }
    return (filePath: '', error: 'Failed to play track', headers: null);
  }

  Future<PreviewTrackResponse> previewTrack(TrackInfo track) async {
    try {
      final body = <String, dynamic>{'id': _newId(), 'track': track.toJson()};
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/playback/preview'),
        headers: {..._authHeaders(), 'Content-Type': 'application/json'},
        body: jsonEncode(body),
      );
      final json = jsonDecode(response.body) as Map<String, dynamic>;
      final error = json['error'] as String?;
      if (response.statusCode == 200) {
        return PreviewTrackResponse(
          id: json['id'] as String,
          url: json['url'] as String?,
          source: json['source'] as String?,
          error: error,
        );
      } else {
        if (error != null) {
          log(
            "Error fetching preview for track ${track.title}: $error",
            isError: true,
          );
        }
        return PreviewTrackResponse(id: '');
      }
    } catch (e) {
      log('previewTrack error: $e', isError: true);
      return PreviewTrackResponse(id: '');
    }
  }

  Future<TrackInfo?> getTrackInfo(String trackId) async {
    try {
      final response = await http.get(
        Uri.parse('$baseUrl/api/tawai/library/tracks/$trackId'),
        headers: _authHeaders(),
      );
      if (response.statusCode == 200) {
        final json = jsonDecode(response.body) as Map<String, dynamic>;
        return TrackInfoJson.fromJson(json);
      }
    } catch (e) {
      log('getTrackInfo error: $e', isError: true);
    }
    return null;
  }

  Future<bool> reportPlayback({
    required String trackId,
    required String playedAt,
    required String source,
  }) async {
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/discovery/lb/report'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({
          'id': _newId(),
          'user_id': SettingsManager.currentUserId.value ?? '',
          'track_id': trackId,
          'played_at': playedAt,
          'source': source,
        }),
      );
      return response.statusCode == 200;
    } catch (e) {
      log('reportPlayback error: $e', isError: true);
      return false;
    }
  }

  Future<List<PlaybackRecord>> getHistory({int limit = 50}) async {
    try {
      final response = await http.get(
        Uri.parse('$baseUrl/api/tawai/playback/history?limit=$limit'),
        headers: _authHeaders(),
      );
      if (response.statusCode == 200) {
        final json = jsonDecode(response.body) as Map<String, dynamic>;
        final records = json['records'] as List<dynamic>;
        return records.map((r) {
          final m = r as Map<String, dynamic>;
          return PlaybackRecord(
            id: m['id'] as String,
            trackId: m['track_id'] as String,
            trackTitle: m['track_title'] as String? ?? '',
            albumTitle: m['album_title'] as String? ?? '',
            artistName: m['artist_name'] as String? ?? '',
            playedAt: m['played_at'] as String? ?? '',
            source: m['source'] as String? ?? '',
            scrobbled: m['scrobbled'] as bool? ?? false,
            durationSecs: (m['duration_secs'] as num?)?.toDouble(),
          );
        }).toList();
      }
    } catch (e) {
      log('getHistory error: $e', isError: true);
    }
    return [];
  }

  // ---------------------------------------------------------------------------
  // Library
  // ---------------------------------------------------------------------------

  Future<List<TrackInfo>> getTracks({String? albumId}) async {
    try {
      final queryParams = <String, String>{};
      if (albumId != null) queryParams['album_id'] = albumId;
      final uri = Uri.parse(
        '$baseUrl/api/tawai/library/tracks',
      ).replace(queryParameters: queryParams.isNotEmpty ? queryParams : null);
      final response = await http.get(uri, headers: _authHeaders());
      if (response.statusCode == 200) {
        final json = jsonDecode(response.body) as Map<String, dynamic>;
        final list = json['tracks'] as List<dynamic>;
        return list.map((e) {
          final m = e as Map<String, dynamic>;
          return TrackInfoJson.fromJson(m);
        }).toList();
      }
    } catch (e) {
      log('getTracks error: $e', isError: true);
    }
    return [];
  }

  Future<List<AlbumInfo>> getAlbums({String? artistId}) async {
    try {
      final queryParams = <String, String>{};
      if (artistId != null) queryParams['artist_id'] = artistId;
      final uri = Uri.parse(
        '$baseUrl/api/tawai/library/albums',
      ).replace(queryParameters: queryParams.isNotEmpty ? queryParams : null);
      final response = await http.get(uri, headers: _authHeaders());
      if (response.statusCode == 200) {
        final json = jsonDecode(response.body) as Map<String, dynamic>;
        final list = json['albums'] as List<dynamic>;
        return list.map((e) {
          final m = e as Map<String, dynamic>;
          return AlbumInfo(
            id: m['id'] as String,
            title: m['title'] as String,
            artistsString: m['artists_string'] as String? ?? '',
            artists: [],
            releaseDate: m['release_date'] as String?,
            trackCount: (m['track_count'] as num?)?.toInt() ?? 0,
          );
        }).toList();
      }
    } catch (e) {
      log('getAlbums error: $e', isError: true);
    }
    return [];
  }

  Future<List<ArtistInfo>> getArtists() async {
    try {
      final response = await http.get(
        Uri.parse('$baseUrl/api/tawai/library/artists'),
        headers: _authHeaders(),
      );
      if (response.statusCode == 200) {
        final json = jsonDecode(response.body) as Map<String, dynamic>;
        final list = json['artists'] as List<dynamic>;
        return list.map((e) {
          final m = e as Map<String, dynamic>;
          return ArtistInfo(
            id: m['id'] as String,
            name: m['name'] as String,
            sortName: m['sort_name'] as String?,
            mbid: m['mbid'] as String?,
            thumbnailUrl: m['thumbnail_url'] as String?,
            albumCount: (m['album_count'] as num?)?.toInt() ?? 0,
            trackCount: (m['track_count'] as num?)?.toInt() ?? 0,
          );
        }).toList();
      }
    } catch (e) {
      log('getArtists error: $e', isError: true);
    }
    return [];
  }

  Future<List<PlaylistInfo>> getPlaylists() async {
    try {
      final response = await http.get(
        Uri.parse('$baseUrl/api/tawai/library/playlists'),
        headers: _authHeaders(),
      );
      if (response.statusCode == 200) {
        final json = jsonDecode(response.body) as Map<String, dynamic>;
        final list = json['playlists'] as List<dynamic>;
        return list.map((e) {
          final m = e as Map<String, dynamic>;
          return PlaylistInfo(
            id: m['id'] as String,
            name: m['name'] as String,
            description: m['description'] as String?,
            isSmart: m['is_smart'] as bool? ?? false,
            trackCount: (m['track_count'] as num?)?.toInt() ?? 0,
            createdAt: m['created_at'] as String?,
          );
        }).toList();
      }
    } catch (e) {
      log('getPlaylists error: $e', isError: true);
    }
    return [];
  }

  Future<String> createPlaylist(String name) async {
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/library/playlists'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({
          'id': _newId(),
          'user_id': SettingsManager.currentUserId.value ?? '',
          'name': name,
        }),
      );
      if (response.statusCode == 200) {
        final data = jsonDecode(response.body);
        return data['playlist_id'] as String;
      }
    } catch (e) {
      log('createPlaylist error: $e', isError: true);
    }
    return '';
  }

  Future<bool> deletePlaylist(String playlistId) async {
    try {
      final response = await http.delete(
        Uri.parse('$baseUrl/api/tawai/library/playlists/$playlistId'),
        headers: _authHeaders(),
      );
      return response.statusCode == 200;
    } catch (e) {
      log('deletePlaylist error: $e', isError: true);
      return false;
    }
  }

  // ---------------------------------------------------------------------------
  // Playlist tracks
  // ---------------------------------------------------------------------------

  Future<List<TrackInfo>> getPlaylistTracks(String playlistId) async {
    try {
      final response = await http.get(
        Uri.parse('$baseUrl/api/tawai/library/playlists/$playlistId/tracks'),
        headers: _authHeaders(),
      );
      if (response.statusCode == 200) {
        final json = jsonDecode(response.body) as Map<String, dynamic>;
        final list = json['tracks'] as List<dynamic>;
        return list.map((e) {
          final m = e as Map<String, dynamic>;
          return TrackInfoJson.fromJson(m);
        }).toList();
      }
    } catch (e) {
      log('getPlaylistTracks error: $e', isError: true);
    }
    return [];
  }

  Future<bool> addTrackToPlaylist(String playlistId, String trackId) async {
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/library/playlists/$playlistId/tracks'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({
          'id': _newId(),
          'playlist_id': playlistId,
          'track_id': trackId,
        }),
      );
      return response.statusCode == 200;
    } catch (e) {
      log('addTrackToPlaylist error: $e', isError: true);
      return false;
    }
  }

  Future<bool> removeTrackFromPlaylist(
    String playlistId,
    String trackId,
  ) async {
    try {
      final response = await http.delete(
        Uri.parse(
          '$baseUrl/api/tawai/library/playlists/$playlistId/tracks/$trackId',
        ),
        headers: _authHeaders(),
      );
      return response.statusCode == 200;
    } catch (e) {
      log('removeTrackFromPlaylist error: $e', isError: true);
      return false;
    }
  }

  Future<bool> reorderPlaylistTracks(
    String playlistId,
    List<String> trackIds,
  ) async {
    try {
      final response = await http.put(
        Uri.parse(
          '$baseUrl/api/tawai/library/playlists/$playlistId/tracks/reorder',
        ),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({
          'id': _newId(),
          'playlist_id': playlistId,
          'track_ids': trackIds,
        }),
      );
      return response.statusCode == 200;
    } catch (e) {
      log('reorderPlaylistTracks error: $e', isError: true);
      return false;
    }
  }

  // ---------------------------------------------------------------------------
  // Downloads (unified)
  // ---------------------------------------------------------------------------

  Future<List<DownloadRecord>> listDownloads({String? source}) async {
    try {
      final queryParams = <String, String>{};
      if (source != null) queryParams['source'] = source;
      final uri = Uri.parse(
        '$baseUrl/api/tawai/download/list',
      ).replace(queryParameters: queryParams.isNotEmpty ? queryParams : null);
      final response = await http.get(uri, headers: _authHeaders());
      if (response.statusCode == 200) {
        final json = jsonDecode(response.body) as Map<String, dynamic>;
        final list = json['downloads'] as List<dynamic>;
        return list.map((e) {
          final m = e as Map<String, dynamic>;
          return DownloadRecord(
            id: m['id'] as String? ?? '',
            source: m['source'] as String? ?? '',
            sourceId: m['source_id'] as String? ?? '',
            url: m['url'] as String? ?? '',
            destPath: m['dest_path'] as String? ?? '',
            filename: m['filename'] as String? ?? '',
            totalSize: (m['total_size'] as num?)?.toInt() ?? 0,
            downloaded: (m['downloaded'] as num?)?.toInt() ?? 0,
            state: m['state'] as String? ?? '',
            error: m['error'] as String? ?? '',
            addedAt: m['added_at'] as String? ?? '',
            updatedAt: m['updated_at'] as String? ?? '',
          );
        }).toList();
      }
    } catch (e) {
      log('listDownloads error: $e', isError: true);
    }
    return [];
  }

  Future<List<DownloadRecord>> pollDownloads() async {
    try {
      final id = _newId();
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/download/poll'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({
          'id': id,
          'user_id': SettingsManager.currentUserId.value ?? '',
        }),
      );
      if (response.statusCode == 200) {
        final json = jsonDecode(response.body) as Map<String, dynamic>;
        final list = json['downloads'] as List<dynamic>;
        return list.map((e) {
          final m = e as Map<String, dynamic>;
          return DownloadRecord(
            id: m['id'] as String? ?? '',
            source: m['source'] as String? ?? '',
            sourceId: m['source_id'] as String? ?? '',
            url: m['url'] as String? ?? '',
            destPath: m['dest_path'] as String? ?? '',
            filename: m['filename'] as String? ?? '',
            totalSize: (m['total_size'] as num?)?.toInt() ?? 0,
            downloaded: (m['downloaded'] as num?)?.toInt() ?? 0,
            state: m['state'] as String? ?? '',
            error: m['error'] as String? ?? '',
            addedAt: m['added_at'] as String? ?? '',
            updatedAt: m['updated_at'] as String? ?? '',
          );
        }).toList();
      }
    } catch (e) {
      log('pollDownloads error: $e', isError: true);
    }
    return [];
  }

  Future<({List<DlSearchItem> results, bool success, String? error})> search(
    String sourceType,
    String query,
  ) async {
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/download/search'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({
          'id': _newId(),
          'source_type': sourceType,
          'query': query,
        }),
      );
      if (response.statusCode == 200) {
        final json = jsonDecode(response.body) as Map<String, dynamic>;
        final resultsList = (json['results'] as List<dynamic>?) ?? [];
        final results = resultsList.map((e) {
          final m = e as Map<String, dynamic>;
          return DlSearchItem(
            filename: m['filename'] as String? ?? '',
            size: Uint64.fromBigInt(BigInt.from(m['size'] as num? ?? 0)),
            sourceType: m['source_type'] as String? ?? '',
            username: m['username'] as String?,
            title: m['title'] as String?,
            thumbnail: m['thumbnail'] as String?,
            duration: (m['duration'] as num?)?.toDouble(),
            channel: m['channel'] as String?,
            bitrate: m['bitrate'] as int?,
            extension: m['extension'] as String?,
          );
        }).toList();
        return (results: results, success: true, error: null);
      }
      final body = jsonDecode(response.body);
      return (
        results: <DlSearchItem>[],
        success: false,
        error: body['error'] as String? ?? 'HTTP ${response.statusCode}',
      );
    } catch (e) {
      log('search error: $e', isError: true);
      return (results: <DlSearchItem>[], success: false, error: e.toString());
    }
  }

  Future<({bool success, String downloadId, String? error})> create(
    String sourceType,
    String url,
    String dest,
    String userId, {
    String? extra,
  }) async {
    try {
      final body = <String, dynamic>{
        'id': _newId(),
        'source_type': sourceType,
        'url': url,
        'dest': dest,
        'user_id': userId,
      };
      if (extra != null) body['extra'] = extra;
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/download/create'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode(body),
      );
      if (response.statusCode == 200) {
        final json = jsonDecode(response.body);
        return (
          success: true,
          downloadId: json['download_id'] as String? ?? '',
          error: null,
        );
      }
      return (
        success: false,
        downloadId: '',
        error: 'HTTP ${response.statusCode}',
      );
    } catch (e) {
      log('create error: $e', isError: true);
      return (success: false, downloadId: '', error: e.toString());
    }
  }

  Future<({bool success, String? error})> cancel(
    String sourceType,
    String downloadId, {
    String? extra,
  }) async {
    try {
      final body = <String, dynamic>{
        'id': _newId(),
        'source_type': sourceType,
        'download_id': downloadId,
      };
      if (extra != null) body['extra'] = extra;
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/download/cancel'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode(body),
      );
      if (response.statusCode == 200) {
        return (success: true, error: null);
      }
      return (success: false, error: 'HTTP ${response.statusCode}');
    } catch (e) {
      log('cancel error: $e', isError: true);
      return (success: false, error: e.toString());
    }
  }

  Future<({bool success, String? version, String? error})> testConnection(
    String sourceType, {
    String? url,
    String? token,
    String? username,
    String? password,
  }) async {
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/download/test-connection'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({
          'id': _newId(),
          'source_type': sourceType,
          'url': url,
          'token': token,
          'username': username,
          'password': password,
        }),
      );
      final json = jsonDecode(response.body) as Map<String, dynamic>;
      final error = json["error"] as String?;
      if (response.statusCode == 200) {
        return (
          success: json['success'] as bool,
          version: json['version'] as String?,
          error: error,
        );
      }
      return (
        success: false,
        version: null,
        error:
            'HTTP ${response.statusCode} ${error != null ? (": ", error) : ""}',
      );
    } catch (e) {
      log('testConnection error: $e', isError: true);
      return (success: false, version: null, error: e.toString());
    }
  }

  Future<String?> getInfo(String sourceType, String url) async {
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/download/get-info'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({
          'id': _newId(),
          'source_type': sourceType,
          'url': url,
        }),
      );
      if (response.statusCode == 200) {
        final json = jsonDecode(response.body) as Map<String, dynamic>;
        return json['info'] as String?;
      }
      return null;
    } catch (e) {
      log('getInfo error: $e', isError: true);
      return null;
    }
  }

  // ---------------------------------------------------------------------------
  // Identify (remote)
  // ---------------------------------------------------------------------------

  Future<List<TrackInfo>> listUnidentifiedTracks({String? sourceId}) async {
    try {
      final uri = sourceId != null
          ? Uri.parse(
              '$baseUrl/api/tawai/library/identify/unidentified?source_id=${Uri.encodeQueryComponent(sourceId)}',
            )
          : Uri.parse('$baseUrl/api/tawai/library/identify/unidentified');
      final response = await http.get(uri, headers: _authHeaders());
      if (response.statusCode == 200) {
        final body = jsonDecode(response.body) as Map<String, dynamic>;
        final list = body['tracks'] as List<dynamic>;
        return list.map((e) {
          final m = e as Map<String, dynamic>;
          return TrackInfoJson.fromJson(m);
        }).toList();
      }
    } catch (e) {
      log('listUnidentifiedTracks error: $e', isError: true);
    }
    return [];
  }

  Future<List<TrackInfo>> listDownloadFolderTracks() async {
    try {
      final uri = Uri.parse(
        '$baseUrl/api/tawai/library/identify/download-folder',
      );
      final response = await http.get(uri, headers: _authHeaders());
      if (response.statusCode == 200) {
        final body = jsonDecode(response.body) as Map<String, dynamic>;
        final list = body['tracks'] as List<dynamic>;
        return list.map((e) {
          final m = e as Map<String, dynamic>;
          return TrackInfoJson.fromJson(m);
        }).toList();
      }
    } catch (e) {
      log('listDownloadFolderTracks error: $e', isError: true);
    }
    return [];
  }

  Future<List<TrackInfo>> listTracksBySource(String sourceId) async {
    try {
      final response = await http.get(
        Uri.parse('$baseUrl/api/tawai/library/tracks/by-source/$sourceId'),
        headers: _authHeaders(),
      );
      if (response.statusCode == 200) {
        final body = jsonDecode(response.body) as Map<String, dynamic>;
        final list = body['tracks'] as List<dynamic>;
        return list.map((e) {
          final m = e as Map<String, dynamic>;
          return TrackInfoJson.fromJson(m);
        }).toList();
      }
    } catch (e) {
      log('listTracksBySource error: $e', isError: true);
    }
    return [];
  }

  Future<String?> getAlbumMbid(String albumId) async {
    try {
      final response = await http.get(
        Uri.parse('$baseUrl/api/tawai/library/tracks/album-mbid/$albumId'),
        headers: _authHeaders(),
      );
      if (response.statusCode == 200) {
        final body = jsonDecode(response.body) as Map<String, dynamic>;
        return body['mbid'] as String?;
      }
    } catch (e) {
      log('getAlbumMbid error: $e', isError: true);
    }
    return null;
  }

  Future<List<MatchCandidate>> identifySingleTrack(String trackId) async {
    try {
      final response = await http.get(
        Uri.parse('$baseUrl/api/tawai/identify/mb/track/$trackId'),
        headers: _authHeaders(),
      );
      if (response.statusCode == 200) {
        final body = jsonDecode(response.body) as Map<String, dynamic>;
        final list = body['candidates'] as List<dynamic>;
        return list.map((e) {
          final m = e as Map<String, dynamic>;
          return MatchCandidate(
            score: (m['score'] as num).toDouble(),
            title: m['title'] as String,
            artist: m['artist'] as String,
            artistId: m['artist_id'] as String?,
            album: m['album'] as String,
            albumId: m['album_id'] as String?,
            recordingId: m['recording_id'] as String?,
            releaseDate: m['release_date'] as String?,
            acoustId: m['acoust_id'] as String?,
          );
        }).toList();
      }
    } catch (e) {
      log('identifySingleTrack error: $e', isError: true);
    }
    return [];
  }

  Future<List<RecordingInfo>> searchMusicBrainz(String query) async {
    try {
      final response = await http.get(
        Uri.parse(
          '$baseUrl/api/tawai/identify/mb/search?query=${Uri.encodeQueryComponent(query)}',
        ),
        headers: _authHeaders(),
      );
      if (response.statusCode == 200) {
        final body = jsonDecode(response.body) as Map<String, dynamic>;
        final list = body['recordings'] as List<dynamic>;
        return list.map((e) {
          final m = e as Map<String, dynamic>;
          return RecordingInfo(
            id: m['id'] as String,
            title: m['title'] as String,
            score: (m['score'] as num).toDouble(),
            artist: m['artist'] as String,
            artistId: m['artist_id'] as String?,
            durationSecs: (m['duration_secs'] as num?)?.toDouble(),
            acoustId: m['acoust_id'] as String?,
            releases: (m['releases'] as List<dynamic>)
                .map(
                  (r) => ReleaseInfo(
                    id: r['id'] as String,
                    title: r['title'] as String,
                    date: r['date'] as String?,
                    country: r['country'] as String?,
                    artist: r['artist'] as String? ?? '',
                    artistId: r['artist_id'] as String?,
                    tracks:
                        (r['tracks'] as List<dynamic>?)
                            ?.map(
                              (t) => ReleaseTrackInfo(
                                id: t['id'] as String,
                                title: t['title'] as String,
                                position: t['position'] as int?,
                                discNumber: (t['disc_number'] as num?)?.toInt(),
                                durationSecs: (t['duration_secs'] as num?)
                                    ?.toDouble(),
                              ),
                            )
                            .toList() ??
                        [],
                  ),
                )
                .toList(),
            cover: m['cover'] as String?,
          );
        }).toList();
      }
    } catch (e) {
      log('searchMusicBrainz error: $e', isError: true);
    }
    return [];
  }

  Future<GetReleaseTracksResponse> getReleaseTracks(String releaseId) async {
    try {
      final response = await http.get(
        Uri.parse('$baseUrl/api/tawai/identify/mb/release/$releaseId/tracks'),
        headers: _authHeaders(),
      );
      if (response.statusCode == 200) {
        final json = jsonDecode(response.body) as Map<String, dynamic>;
        final tracksList = (json['tracks'] as List<dynamic>).map((e) {
          final m = e as Map<String, dynamic>;
          return ReleaseTrackInfo(
            id: m['id'] as String,
            title: m['title'] as String,
            position: m['position'] as int?,
            discNumber: (m['disc_number'] as num?)?.toInt(),
            durationSecs: (m['duration_secs'] as num?)?.toDouble(),
          );
        }).toList();
        return GetReleaseTracksResponse(
          id: '',
          releaseId: json['release_id'] as String? ?? releaseId,
          releaseTitle: json['release_title'] as String? ?? '',
          releaseDate: json['release_date'] as String?,
          artist: json['artist'] as String? ?? '',
          artistId: json['artist_id'] as String?,
          tracks: tracksList,
        );
      }
    } catch (e) {
      log('getReleaseTracks error: $e', isError: true);
    }
    return GetReleaseTracksResponse(
      id: '',
      releaseId: releaseId,
      releaseTitle: '',
      releaseDate: null,
      artist: '',
      artistId: null,
      tracks: [],
    );
  }

  Future<RecordingInfo?> fetchRecording(String mbid) async {
    try {
      final response = await http.get(
        Uri.parse('$baseUrl/api/tawai/identify/mb/recording/$mbid'),
        headers: _authHeaders(),
      );
      if (response.statusCode == 200) {
        final body = jsonDecode(response.body) as Map<String, dynamic>;
        final recording = body['recording'];
        if (recording == null) return null;
        final json = recording as Map<String, dynamic>;
        return RecordingInfo(
          id: json['id'] as String,
          title: json['title'] as String,
          score: (json['score'] as num?)?.toDouble() ?? 0,
          artist: json['artist'] as String? ?? '',
          artistId: json['artist_id'] as String?,
          durationSecs: (json['duration_secs'] as num?)?.toDouble(),
          acoustId: json['acoust_id'] as String?,
          releases:
              (json['releases'] as List<dynamic>?)
                  ?.map(
                    (r) => ReleaseInfo(
                      id: r['id'] as String,
                      title: r['title'] as String,
                      date: r['date'] as String?,
                      country: r['country'] as String?,
                      artist: r['artist'] as String? ?? '',
                      artistId: r['artist_id'] as String?,
                      tracks:
                          (r['tracks'] as List<dynamic>?)
                              ?.map(
                                (t) => ReleaseTrackInfo(
                                  id: t['id'] as String,
                                  title: t['title'] as String,
                                  position: t['position'] as int?,
                                  durationSecs: (t['duration_secs'] as num?)
                                      ?.toDouble(),
                                ),
                              )
                              .toList() ??
                          [],
                    ),
                  )
                  .toList() ??
              [],
          cover: json['cover'] as String?,
        );
      }
    } catch (e) {
      log('fetchRecording error: $e', isError: true);
    }
    return null;
  }

  Future<({bool success, String? error, String? newFilePath})>
  applyIdentification({
    String? userId,
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
    try {
      final body = <String, dynamic>{
        'id': _newId(),
        'track_id': trackId,
        'title': title,
        'artist': artist,
        'album': album,
      };
      if (artistMbid != null) body['artist_mbid'] = artistMbid;
      if (albumMbid != null) body['album_mbid'] = albumMbid;
      if (albumDisambiguation != null) {
        body['album_disambiguation'] = albumDisambiguation;
      }
      if (releaseDate != null) body['release_date'] = releaseDate;
      if (trackNum != null) body['track_num'] = trackNum;
      if (discNum != null) body['disc_num'] = discNum;
      if (mbidRecording != null) body['mbid_recording'] = mbidRecording;
      if (lyrics != null) body['lyrics'] = lyrics;
      if (coverBytes != null) body['cover_bytes'] = base64Encode(coverBytes);
      if (totalDiscs > 0) body['total_discs'] = totalDiscs;
      if (filePath != null) body['file_path'] = filePath;
      if (targetSourceId != null) body['source_id'] = targetSourceId;

      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/library/identify/apply'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode(body),
      );
      if (response.statusCode == 200) {
        final data = jsonDecode(response.body) as Map<String, dynamic>;
        return (
          success: true,
          error: null,
          newFilePath: data['new_file_path'] as String?,
        );
      }
      final data = jsonDecode(response.body);
      return (
        success: false,
        error: data['error'] as String? ?? 'HTTP ${response.statusCode}',
        newFilePath: null,
      );
    } catch (e) {
      log('applyIdentification error: $e', isError: true);
      return (success: false, error: e.toString(), newFilePath: null);
    }
  }

  Future<RecordingInfo?> fingerprintTrack(
    String? trackId, {
    String? filePath,
  }) async {
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/identify/mb/fingerprint'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({
          'id': _newId(),
          'track_id': trackId,
          'file_path': filePath,
        }),
      );
      return _parseFingerprintResponse(response);
    } catch (e) {
      log('fingerprintTrack error: $e', isError: true);
    }
    return null;
  }

  RecordingInfo? _parseFingerprintResponse(http.Response response) {
    if (response.statusCode != 200) return null;
    final json = jsonDecode(response.body);
    if (json == null) return null;
    final recording = json['recording'];
    if (recording == null) return null;
    return _parseRecordingInfo(recording as Map<String, dynamic>);
  }

  RecordingInfo _parseRecordingInfo(Map<String, dynamic> json) {
    return RecordingInfo(
      id: json['id'] as String,
      title: json['title'] as String,
      score: (json['score'] as num).toDouble(),
      artist: json['artist'] as String,
      artistId: json['artist_id'] as String?,
      durationSecs: (json['duration_secs'] as num?)?.toDouble(),
      acoustId: json['acoust_id'] as String?,
      releases:
          (json['releases'] as List<dynamic>?)
              ?.map(
                (r) => ReleaseInfo(
                  id: r['id'] as String,
                  title: r['title'] as String,
                  date: r['date'] as String?,
                  country: r['country'] as String?,
                  artist: r['artist'] as String? ?? '',
                  artistId: r['artist_id'] as String?,
                  tracks:
                      (r['tracks'] as List<dynamic>?)
                          ?.map(
                            (t) => ReleaseTrackInfo(
                              id: t['id'] as String,
                              title: t['title'] as String,
                              position: t['position'] as int?,
                              durationSecs: (t['duration_secs'] as num?)
                                  ?.toDouble(),
                            ),
                          )
                          .toList() ??
                      [],
                ),
              )
              .toList() ??
          [],
      cover: json['cover'] as String?,
    );
  }

  Future<LyricsResult?> fetchLyrics({
    required String title,
    required String artist,
    String? album,
    double? duration,
    bool preferSync = true,
  }) async {
    try {
      final params = <String, String>{
        'title': title,
        'artist': artist,
        'prefer_sync': preferSync.toString(),
      };
      if (album != null) params['album'] = album;
      if (duration != null) params['duration'] = duration.toString();
      final uri = Uri.parse(
        '$baseUrl/api/tawai/library/identify/lyrics',
      ).replace(queryParameters: params);
      final response = await http.get(uri, headers: _authHeaders());
      if (response.statusCode == 200) {
        final json = jsonDecode(response.body) as Map<String, dynamic>;
        return LyricsResult(
          id: Uint64.fromBigInt(BigInt.from(json['id'] as int)),
          title: json['title'] as String,
          artist: json['artist'] as String,
          album: json['album'] as String,
          duration: (json['duration'] as num).toDouble(),
          instrumental: json['instrumental'] as bool,
          lyrics: json['lyrics'] as String,
          synced: json['synced'] as bool,
        );
      }
    } catch (e) {
      log('fetchLyrics error: $e', isError: true);
    }
    return null;
  }

  Future<List<LyricsResult>> searchLyrics({
    required String query,
    bool preferSync = true,
  }) async {
    try {
      final uri = Uri.parse('$baseUrl/api/tawai/library/identify/lyrics/search')
          .replace(
            queryParameters: {
              'query': query,
              'prefer_sync': preferSync.toString(),
            },
          );
      final response = await http.get(uri, headers: _authHeaders());
      if (response.statusCode == 200) {
        final json = jsonDecode(response.body) as List<dynamic>;
        return json.map((j) {
          final m = j as Map<String, dynamic>;
          return LyricsResult(
            id: Uint64.fromBigInt(BigInt.from(m['id'] as int)),
            title: m['title'] as String,
            artist: m['artist'] as String,
            album: m['album'] as String,
            duration: (m['duration'] as num).toDouble(),
            instrumental: m['instrumental'] as bool,
            lyrics: m['lyrics'] as String,
            synced: m['synced'] as bool,
          );
        }).toList();
      }
    } catch (e) {
      log('searchLyrics error: $e', isError: true);
    }
    return [];
  }

  // ---------------------------------------------------------------------------
  // Auth helpers
  // ---------------------------------------------------------------------------

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
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/account/update'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({
          'id': _newId(),
          'operator_user_id': operatorUserId,
          'operator_password': operatorPassword,
          'target_user_id': targetUserId,
          'new_username': newUsername,
          'new_password': newPassword,
          'display_name': newDisplayName,
          'role': role,
        }),
      );
      if (response.statusCode == 200) {
        final data = jsonDecode(response.body);
        _jwt = data['access_token'] as String? ?? '';
        _csrfToken = data['csrf_token'] as String? ?? '';
        await SettingsManager.loadFromBackend();
        return (
          success: true,
          userId: data['user_id'] as String? ?? '',
          username: data['username'] as String? ?? newUsername ?? '',
          displayName: data['display_name'] as String? ?? '',
          role: data['role'] as String? ?? 'user',
          apiKey: data['api_key'] as String? ?? '',
        );
      }
      log(
        'Update account failed: ${response.statusCode} ${response.body}',
        isError: true,
      );
    } catch (e) {
      log("Update account error: $e", isError: true);
    }
    return (
      success: false,
      userId: '',
      username: newUsername ?? '',
      displayName: '',
      role: 'user',
      apiKey: '',
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
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/account/create'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({
          'admin_username': adminUsername,
          'admin_password': adminPassword,
          'username': username,
          'password': password,
          'display_name': displayName,
          'role': role,
        }),
      );
      if (response.statusCode == 200) {
        final data = jsonDecode(response.body);
        return (
          success: data['success'] as bool? ?? true,
          userId: data['user_id'] as String? ?? '',
          username: data['username'] as String? ?? username,
          displayName: data['display_name'] as String? ?? '',
          role: data['role'] as String? ?? 'user',
          apiKey: data['api_key'] as String? ?? '',
        );
      }
      log(
        'Create account failed: ${response.statusCode} ${response.body}',
        isError: true,
      );
    } catch (e) {
      log("Create account error: $e", isError: true);
    }
    return (
      success: false,
      userId: '',
      username: username,
      displayName: '',
      role: 'user',
      apiKey: '',
    );
  }

  Future<List<UserListItem>> listUsers() async {
    try {
      final response = await http.get(
        Uri.parse('$baseUrl/api/tawai/account/list'),
        headers: _authHeaders(),
      );
      if (response.statusCode == 200) {
        final data = jsonDecode(response.body);
        final users = data['users'] as List<dynamic>? ?? [];
        return users
            .map(
              (u) => UserListItem(
                id: (u['id'] as String?) ?? '',
                username: (u['username'] as String?) ?? '',
                displayName: (u['display_name'] as String?) ?? '',
                role: (u['role'] as String?) ?? 'user',
                apiKey: (u['api_key'] as String?) ?? '',
              ),
            )
            .toList();
      }
      log(
        'List users failed: ${response.statusCode} ${response.body}',
        isError: true,
      );
    } catch (e) {
      log("List users error: $e", isError: true);
    }
    return [];
  }

  Future<({bool success, String username})> deleteUser({
    required String adminUsername,
    required String adminPassword,
    required String targetUsername,
  }) async {
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/account/delete'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({
          'admin_username': adminUsername,
          'admin_password': adminPassword,
          'target_username': targetUsername,
        }),
      );
      if (response.statusCode == 200) {
        final data = jsonDecode(response.body);
        return (
          success: data['success'] as bool? ?? true,
          username: data['username'] as String? ?? targetUsername,
        );
      }
      log(
        'Delete account failed: ${response.statusCode} ${response.body}',
        isError: true,
      );
    } catch (e) {
      log("Delete account error: $e", isError: true);
    }
    return (success: false, username: targetUsername);
  }

  Future<bool> verifyPassword(String password) async {
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/auth/verify-password'),
        headers: _authHeaders(extra: {'X-Password': password}),
      );
      if (response.statusCode == 200) {
        return true;
      }
      log(
        'Verify password failed: ${response.statusCode} ${response.body}',
        isError: true,
      );
    } catch (e) {
      log("Verify password error: $e", isError: true);
    }
    return false;
  }

  Future<String?> encrypt(String plainText) async {
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/auth/encrypt'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({'id': _newId(), 'plain_key': plainText}),
      );
      if (response.statusCode == 200) {
        final data = jsonDecode(response.body);
        return data['encrypted_key'] as String?;
      }
    } catch (e) {
      log("Encrypt error: $e", isError: true);
    }
    return null;
  }

  Future<String?> decrypt(String encryptedText) async {
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/auth/decrypt'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({'id': _newId(), 'encrypted_key': encryptedText}),
      );
      if (response.statusCode == 200) {
        final data = jsonDecode(response.body);
        return data['decrypted_key'] as String?;
      }
    } catch (e) {
      log("Decrypt error: $e", isError: true);
    }
    return null;
  }

  // ---------------------------------------------------------------------------
  // Library Sources (remote)
  // ---------------------------------------------------------------------------

  Future<List<LibrarySourceInfo>> listLibrarySources() async {
    try {
      final response = await http.get(
        Uri.parse('$baseUrl/api/tawai/library/sources'),
        headers: _authHeaders(),
      );
      if (response.statusCode == 200) {
        final data = jsonDecode(response.body);
        final list = data['sources'] as List<dynamic>;
        return list.map((e) {
          final m = e as Map<String, dynamic>;
          return LibrarySourceInfo(
            id: m['id'] as String,
            sourceType: m['source_type'] as String,
            url: m['url'] as String,
            name: m['name'] as String,
            lastSyncAt: m['last_sync_at'] as String?,
            ownerId: m['owner_id'] as String,
            accessRule: m['access_rule'] as String,
            createdAt: m['created_at'] as String,
            updatedAt: m['updated_at'] as String,
          );
        }).toList();
      }
    } catch (e) {
      log("Error listing library sources: $e", isError: true);
    }
    return [];
  }

  Future<List<LibrarySourceInfo>> listEditableSources() async {
    try {
      final response = await http.get(
        Uri.parse('$baseUrl/api/tawai/library/sources/editable'),
        headers: _authHeaders(),
      );
      if (response.statusCode == 200) {
        final data = jsonDecode(response.body);
        final list = data['sources'] as List<dynamic>;
        return list.map((e) {
          final m = e as Map<String, dynamic>;
          return LibrarySourceInfo(
            id: m['id'] as String,
            sourceType: m['source_type'] as String,
            url: m['url'] as String,
            name: m['name'] as String,
            lastSyncAt: m['last_sync_at'] as String?,
            ownerId: m['owner_id'] as String,
            accessRule: m['access_rule'] as String,
            createdAt: m['created_at'] as String,
            updatedAt: m['updated_at'] as String,
          );
        }).toList();
      }
    } catch (e) {
      log("Error listing editable sources: $e", isError: true);
    }
    return [];
  }

  Future<({String sourceId, bool success})> addLibrarySource(
    String url,
    String name,
    String sourceType,
  ) async {
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/library/sources'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({
          'id': _newId(),
          'user_id': SettingsManager.currentUserId.value ?? '',
          'url': url,
          'name': name,
          'source_type': sourceType,
        }),
      );
      if (response.statusCode == 200) {
        final data = jsonDecode(response.body);
        return (sourceId: data['source_id'] as String, success: true);
      }
      return (sourceId: '', success: false);
    } catch (e) {
      log("Error adding library source: $e", isError: true);
      return (sourceId: '', success: false);
    }
  }

  Future<bool> removeLibrarySource(String sourceId) async {
    try {
      final response = await http.delete(
        Uri.parse('$baseUrl/api/tawai/library/sources/$sourceId'),
        headers: _authHeaders(),
      );
      return response.statusCode == 200;
    } catch (e) {
      log("Error removing library source: $e", isError: true);
      return false;
    }
  }

  Future<List<JellyfinLibraryInfo>> testJellyfinSource(String url) async {
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/library/sources/test-jellyfin'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({'id': _newId(), 'url': url}),
      );
      if (response.statusCode == 200) {
        final data = jsonDecode(response.body);
        final list = data['libraries'] as List<dynamic>;
        return list.map((e) {
          final m = e as Map<String, dynamic>;
          return JellyfinLibraryInfo(
            id: m['id'] as String,
            name: m['name'] as String,
          );
        }).toList();
      }
      final data = jsonDecode(response.body);
      throw Exception(data['error'] as String? ?? 'Unknown error');
    } catch (e) {
      log("Error testing Jellyfin source: $e", isError: true);
      rethrow;
    }
  }

  // ---------------------------------------------------------------------------
  // Scan (remote — start ack + status polling)
  // ---------------------------------------------------------------------------

  Future<({bool started, String? error})> scanLibrary({
    required bool force,
  }) async {
    final id = _newId();
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/library/scan'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({'id': id, 'force': force}),
      );
      if (response.statusCode != 200) {
        return (started: false, error: 'HTTP ${response.statusCode}');
      }
      final data = jsonDecode(response.body) as Map<String, dynamic>;
      return (
        started: data['started'] as bool? ?? false,
        error: data['error'] as String?,
      );
    } catch (e) {
      log("Scan start error: $e", isError: true);
      return (started: false, error: 'Request failed');
    }
  }

  // ---------------------------------------------------------------------------
  // Scan status (remote polling)
  // ---------------------------------------------------------------------------

  Future<({bool running, ScanProgressSignal? progress})> getScanStatus() async {
    final id = _newId();
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/library/scan/status'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({'id': id}),
      );
      if (response.statusCode != 200) return (running: false, progress: null);
      final data = jsonDecode(response.body) as Map<String, dynamic>;
      final progress = data['progress'] as Map<String, dynamic>?;
      return (
        running: data['running'] as bool? ?? false,
        progress: progress == null
            ? null
            : ScanProgressSignal(
                id: '',
                currentFile: progress['current_file'] as String? ?? '',
                filesScanned: (progress['files_scanned'] as num?)?.toInt() ?? 0,
                totalFiles: (progress['total_files'] as num?)?.toInt() ?? 0,
                stage: progress['stage'] as String? ?? '',
                complete: progress['complete'] as bool? ?? false,
                tracksFound: (progress['tracks_found'] as num?)?.toInt() ?? 0,
                newTracks: (progress['new_tracks'] as num?)?.toInt() ?? 0,
                duplicates: (progress['duplicates'] as num?)?.toInt() ?? 0,
                deleted: (progress['deleted'] as num?)?.toInt() ?? 0,
                currentSource: progress['current_source'] as String? ?? '',
                error: progress['error'] as String?,
              ),
      );
    } catch (_) {
      return (running: false, progress: null);
    }
  }

  // ---------------------------------------------------------------------------
  // Scan single source (remote SSE)
  // ---------------------------------------------------------------------------

  Future<({bool started, String? error})> scanSource({
    required String sourceId,
    required bool force,
  }) async {
    try {
      final client = http.Client();
      final request =
          http.Request(
              'POST',
              Uri.parse('$baseUrl/api/tawai/library/scan/source'),
            )
            ..headers.addAll(
              _authHeaders(extra: {'Content-Type': 'application/json'}),
            )
            ..body = jsonEncode({
              'id': _newId(),
              'user_id': SettingsManager.currentUserId.value ?? '',
              'source_id': sourceId,
              'force': force,
            });
      final streamed = await client.send(request);
      if (streamed.statusCode != 200) {
        client.close();
        return (started: false, error: 'HTTP ${streamed.statusCode}');
      }

      final completer = Completer<({bool started, String? error})>();
      final buffer = StringBuffer();
      StreamSubscription<String>? subscription;
      subscription = streamed.stream
          .transform(utf8.decoder)
          .listen(
            (chunk) {
              buffer.write(chunk);
              if (!buffer.toString().contains('\n\n')) return;
              String? dataLine;
              for (final line in buffer.toString().split('\n')) {
                if (line.startsWith('data:')) {
                  dataLine = line;
                  break;
                }
              }
              if (dataLine == null) return;
              final data = dataLine
                  .replaceFirst(RegExp(r'^data:\s*'), '')
                  .trim();
              if (data.isEmpty) return;
              final json = jsonDecode(data) as Map<String, dynamic>;
              final complete = json['complete'] as bool? ?? false;
              final error = json['error'] as String?;
              if (complete && error != null) {
                completer.complete((started: false, error: error));
              } else {
                completer.complete((started: true, error: null));
              }
              subscription?.cancel();
              client.close();
            },
            onError: (Object e) {
              if (!completer.isCompleted) {
                completer.complete((started: false, error: 'Stream error'));
              }
            },
          );

      return completer.future.timeout(
        const Duration(seconds: 60),
        onTimeout: () {
          subscription?.cancel();
          client.close();
          return (started: false, error: 'Timeout');
        },
      );
    } catch (e) {
      log('scanSource error: $e', isError: true);
      return (started: false, error: 'Request failed');
    }
  }

  // ---------------------------------------------------------------------------
  // Tag editor (file tags read/write via REST)
  // ---------------------------------------------------------------------------

  Future<ReadFileTagsResponse?> readFileTags(String path) async {
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/library/identify/tags/read'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({'id': _newId(), 'path': path}),
      );
      if (response.statusCode != 200) return null;
      final data = jsonDecode(response.body) as Map<String, dynamic>;
      return ReadFileTagsResponse(
        id: '',
        title: data['title'] as String? ?? '',
        artist: data['artist'] as String? ?? '',
        album: data['album'] as String? ?? '',
        albumArtist: data['album_artist'] as String? ?? '',
        genres:
            (data['genres'] as List<dynamic>?)
                ?.map((e) => e as String)
                .toList() ??
            [],
        trackNumber: (data['track_number'] as num?)?.toInt() ?? 0,
        discNumber: (data['disc_number'] as num?)?.toInt() ?? 0,
        releaseDate: data['release_date'] as String?,
        lyrics: data['lyrics'] as String?,
        cover: data['cover'] != null
            ? base64Decode(data['cover'] as String)
            : null,
        durationSecs: (data['duration_secs'] as num?)?.toDouble() ?? 0.0,
        error: data['error'] as String?,
      );
    } catch (e) {
      log('readFileTags error: $e', isError: true);
      return null;
    }
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
  }) async {
    try {
      final body = <String, dynamic>{
        'id': _newId(),
        'path': path,
        'title': title,
        'artist': artist,
        'album': album,
        'album_artist': albumArtist,
        'genres': genres,
        'track_number': trackNumber,
        'disc_number': discNumber,
      };
      if (releaseDate != null) body['release_date'] = releaseDate;
      if (lyrics != null) body['lyrics'] = lyrics;
      if (cover != null) body['cover'] = base64Encode(cover);

      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/library/identify/tags/write'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode(body),
      );
      if (response.statusCode == 200) {
        final data = jsonDecode(response.body);
        return (
          success: data['success'] as bool? ?? true,
          error: data['error'] as String?,
        );
      }
      final data = jsonDecode(response.body);
      return (
        success: false,
        error: data['error'] as String? ?? 'HTTP ${response.statusCode}',
      );
    } catch (e) {
      log('writeFileTags error: $e', isError: true);
      return (success: false, error: e.toString());
    }
  }

  Future<ReadFileTagsResponse?> readFileTagsBytes(
    String filename,
    Uint8List bytes,
  ) async {
    try {
      final request =
          http.MultipartRequest(
              'POST',
              Uri.parse('$baseUrl/api/tawai/library/identify/tags/read-bytes'),
            )
            ..headers.addAll(_authHeaders())
            ..fields['id'] = _newId()
            ..fields['filename'] = filename
            ..files.add(
              http.MultipartFile.fromBytes('file', bytes, filename: filename),
            );
      final streamed = await request.send();
      final response = await http.Response.fromStream(streamed);
      if (response.statusCode != 200) return null;
      final data = jsonDecode(response.body) as Map<String, dynamic>;
      return ReadFileTagsResponse(
        id: '',
        title: data['title'] as String? ?? '',
        artist: data['artist'] as String? ?? '',
        album: data['album'] as String? ?? '',
        albumArtist: data['album_artist'] as String? ?? '',
        genres:
            (data['genres'] as List<dynamic>?)
                ?.map((e) => e as String)
                .toList() ??
            [],
        trackNumber: (data['track_number'] as num?)?.toInt() ?? 0,
        discNumber: (data['disc_number'] as num?)?.toInt() ?? 0,
        releaseDate: data['release_date'] as String?,
        lyrics: data['lyrics'] as String?,
        cover: data['cover'] != null
            ? base64Decode(data['cover'] as String)
            : null,
        durationSecs: (data['duration_secs'] as num?)?.toDouble() ?? 0.0,
        error: data['error'] as String?,
      );
    } catch (e) {
      log('readFileTagsBytes error: $e', isError: true);
      return null;
    }
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
  }) async {
    try {
      final request =
          http.MultipartRequest(
              'POST',
              Uri.parse('$baseUrl/api/tawai/library/identify/tags/write-bytes'),
            )
            ..headers.addAll(_authHeaders())
            ..fields.addAll({
              'id': _newId(),
              'filename': filename,
              'title': title,
              'artist': artist,
              'album': album,
              'album_artist': albumArtist,
              'track_number': trackNumber.toString(),
              'disc_number': discNumber.toString(),
              for (final g in genres) 'genres': g,
            })
            ..files.add(
              http.MultipartFile.fromBytes('file', bytes, filename: filename),
            );
      if (releaseDate != null && releaseDate.isNotEmpty) {
        request.fields['release_date'] = releaseDate;
      }
      if (lyrics != null && lyrics.isNotEmpty) {
        request.fields['lyrics'] = lyrics;
      }
      if (cover != null) {
        request.files.add(
          http.MultipartFile.fromBytes('cover', cover, filename: 'cover'),
        );
      }
      final streamed = await request.send();
      final response = await http.Response.fromStream(streamed);
      if (response.statusCode == 200) {
        return (success: true, bytes: response.bodyBytes, error: null);
      }
      final data = jsonDecode(response.body);
      return (
        success: false,
        bytes: null,
        error: data['error'] as String? ?? 'HTTP ${response.statusCode}',
      );
    } catch (e) {
      log('writeFileTagsBytes error: $e', isError: true);
      return (success: false, bytes: null, error: e.toString());
    }
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
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/tools/format-naming-preview'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({
          'id': _newId(),
          'pattern': pattern,
          'title': title,
          'artist': artist,
          'album_artist': albumArtist,
          'album': album,
          if (releaseDate != null) 'release_date': releaseDate,
          'track_number': trackNumber,
          'disc_number': discNumber,
          if (albumDisambiguation != null)
            'album_disambiguation': albumDisambiguation,
          'total_discs': totalDiscs,
        }),
      );
      if (response.statusCode == 200) {
        final data = jsonDecode(response.body);
        return data['result'] as String;
      }
      return '';
    } catch (e) {
      log('formatNamingPreview error: $e', isError: true);
      return '';
    }
  }

  // ---------------------------------------------------------------------------
  // Rename
  // ---------------------------------------------------------------------------

  Future<BatchRenamePreviewResponse?> batchRenamePreview({
    required List<String> filePaths,
    required String pattern,
    String? sourceId,
  }) async {
    try {
      final body = <String, dynamic>{
        'id': _newId(),
        'file_paths': filePaths,
        'pattern': pattern,
      };
      if (sourceId != null) body['source_id'] = sourceId;
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/tools/rename-preview'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode(body),
      );
      if (response.statusCode != 200) return null;
      final data = jsonDecode(response.body) as Map<String, dynamic>;
      final previews = (data['previews'] as List<dynamic>)
          .map(
            (e) => RenamePreview(
              filePath: e['file_path'] as String,
              expectedPath: e['expected_path'] as String,
              trackId: e['track_id'] as String? ?? '',
            ),
          )
          .toList();
      return BatchRenamePreviewResponse(
        id: '',
        previews: previews,
        error: data['error'] as String?,
      );
    } catch (e) {
      log('batchRenamePreview error: $e', isError: true);
      return null;
    }
  }

  Future<BatchRenameApplyResponse?> batchRenameApply({
    required List<String> filePaths,
    required List<String> trackIds,
    required String pattern,
  }) async {
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/tools/rename-apply'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({
          'id': _newId(),
          'file_paths': filePaths,
          'track_ids': trackIds,
          'pattern': pattern,
        }),
      );
      if (response.statusCode != 200) return null;
      final data = jsonDecode(response.body) as Map<String, dynamic>;
      final results = (data['results'] as List<dynamic>)
          .map(
            (e) => RenamePreview(
              filePath: e['file_path'] as String,
              expectedPath: e['expected_path'] as String,
              trackId: e['track_id'] as String? ?? '',
            ),
          )
          .toList();
      return BatchRenameApplyResponse(
        id: '',
        results: results,
        error: data['error'] as String?,
      );
    } catch (e) {
      log('batchRenameApply error: $e', isError: true);
      return null;
    }
  }

  Future<CheckNamingConventionResponse?> checkNamingConvention({
    String? sourceId,
    required String pattern,
  }) async {
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/tools/check-convention'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({
          'id': _newId(),
          'pattern': pattern,
          if (sourceId != null) 'source_id': sourceId,
        }),
      );
      if (response.statusCode != 200) return null;
      final data = jsonDecode(response.body) as Map<String, dynamic>;
      final violations = (data['violations'] as List<dynamic>)
          .map(
            (e) => NamingViolation(
              filePath: e['file_path'] as String,
              fileName: e['file_name'] as String,
              expectedName: e['expected_name'] as String,
              trackId: e['track_id'] as String,
            ),
          )
          .toList();
      return CheckNamingConventionResponse(
        id: '',
        violations: violations,
        error: data['error'] as String?,
      );
    } catch (e) {
      log('checkNamingConvention error: $e', isError: true);
      return null;
    }
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
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/tools/missing-metadata'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({
          'id': _newId(),
          'check_title': checkTitle,
          'check_artist': checkArtist,
          'check_album': checkAlbum,
          'check_genre': checkGenre,
          'check_year': checkYear,
          'check_track_number': checkTrackNumber,
          'check_cover': checkCover,
        }),
      );
      if (response.statusCode != 200) return null;
      final data = jsonDecode(response.body) as Map<String, dynamic>;
      final tracksJson = data['tracks'] as List<dynamic>;
      return FindMissingMetadataResponse(
        id: '',
        tracks: tracksJson
            .map(
              (e) => MissingMetadataEntry(
                trackId: e['track_id'] as String,
                filePath: e['file_path'] as String,
                title: e['title'] as String,
                artist: e['artist'] as String,
                album: e['album'] as String,
                missingFields: (e['missing_fields'] as List<dynamic>)
                    .cast<String>(),
              ),
            )
            .toList(),
        error: data['error'] as String?,
      );
    } catch (e) {
      log('findMissingMetadata error: $e', isError: true);
      return null;
    }
  }

  Future<GetLibraryStatsResponse?> getLibraryStats({
    String? namingPattern,
  }) async {
    try {
      final uri = Uri.parse('$baseUrl/api/tawai/tools/stats').replace(
        queryParameters: namingPattern != null
            ? {'naming_pattern': namingPattern}
            : null,
      );
      final response = await http.get(uri, headers: _authHeaders());
      if (response.statusCode != 200) return null;
      final data = jsonDecode(response.body) as Map<String, dynamic>;
      final statsJson = data['stats'] as Map<String, dynamic>?;
      return GetLibraryStatsResponse(
        id: '',
        stats: statsJson != null ? _parseLibraryStats(statsJson) : null,
        error: data['error'] as String?,
      );
    } catch (e) {
      log('getLibraryStats error: $e', isError: true);
      return null;
    }
  }

  LibraryStats _parseLibraryStats(Map<String, dynamic> json) {
    return LibraryStats(
      totalTracks: (json['total_tracks'] as num).toInt(),
      totalAlbums: (json['total_albums'] as num).toInt(),
      totalArtists: (json['total_artists'] as num).toInt(),
      totalDurationSecs: (json['total_duration_secs'] as num).toDouble(),
      averageBitrate: (json['average_bitrate'] as num?)?.toDouble(),
      mostCommonGenre: json['most_common_genre'] as String?,
      genreCount: (json['genre_count'] as num).toInt(),
      formatBreakdown: (json['format_breakdown'] as List<dynamic>)
          .map(
            (e) => FormatEntry(
              format: e['format'] as String,
              count: (e['count'] as num).toInt(),
            ),
          )
          .toList(),
      decadeDistribution: (json['decade_distribution'] as List<dynamic>)
          .map(
            (e) => DecadeEntry(
              decade: e['decade'] as String,
              count: (e['count'] as num).toInt(),
            ),
          )
          .toList(),
      largestAlbumTitle: json['largest_album_title'] as String?,
      largestAlbumTracks: (json['largest_album_tracks'] as num).toInt(),
      mostProlificArtist: json['most_prolific_artist'] as String?,
      mostProlificArtistTracks: (json['most_prolific_artist_tracks'] as num)
          .toInt(),
      namingConformityPct: (json['naming_conformity_pct'] as num?)?.toDouble(),
      totalFileSize: (json['total_file_size'] as num?)?.toInt() ?? 0,
      tracksWithCover: (json['tracks_with_cover'] as num?)?.toInt() ?? 0,
      tracksWithoutCover: (json['tracks_without_cover'] as num?)?.toInt() ?? 0,
      tracksWithLyrics: (json['tracks_with_lyrics'] as num?)?.toInt() ?? 0,
      tracksWithoutLyrics:
          (json['tracks_without_lyrics'] as num?)?.toInt() ?? 0,
      averageTrackDurationSecs:
          (json['average_track_duration_secs'] as num?)?.toDouble() ?? 0,
      shortestTrackTitle: json['shortest_track_title'] as String?,
      shortestTrackDuration: (json['shortest_track_duration'] as num?)
          ?.toDouble(),
      longestTrackTitle: json['longest_track_title'] as String?,
      longestTrackDuration: (json['longest_track_duration'] as num?)
          ?.toDouble(),
      tracksPerAlbumAvg:
          (json['tracks_per_album_avg'] as num?)?.toDouble() ?? 0,
      tracksPerArtistAvg:
          (json['tracks_per_artist_avg'] as num?)?.toDouble() ?? 0,
      tracksWithMbid: (json['tracks_with_mbid'] as num?)?.toInt() ?? 0,
      oldestYear: json['oldest_year'] as String?,
      newestYear: json['newest_year'] as String?,
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
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/tools/write-lyrics'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({
          'id': _newId(),
          'track_id': trackId,
          'lyrics': lyrics,
          'synced': synced,
        }),
      );
      if (response.statusCode != 200) return null;
      final data = jsonDecode(response.body) as Map<String, dynamic>;
      return WriteTrackLyricsResponse(
        id: '',
        success: data['success'] as bool,
        error: data['error'] as String?,
      );
    } catch (e) {
      log('writeTrackLyrics error: $e', isError: true);
      return null;
    }
  }

  // ---------------------------------------------------------------------------
  // Romajize Lyrics
  // ---------------------------------------------------------------------------

  Future<RomajizeLyricsResponse?> romajizeLyrics({
    required String lyrics,
    required bool synced,
    String? lang,
  }) async {
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/tools/romajize-lyrics'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({
          'id': _newId(),
          'lyrics': lyrics,
          'synced': synced,
          'lang': ?lang,
        }),
      );
      if (response.statusCode != 200) return null;
      final data = jsonDecode(response.body) as Map<String, dynamic>;
      return RomajizeLyricsResponse(
        id: '',
        romajized: data['romajized'] as String,
        synced: data['synced'] as bool,
        error: data['error'] as String?,
      );
    } catch (e) {
      log('romajizeLyrics error: $e', isError: true);
      return null;
    }
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
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/tools/find-duplicates'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({
          'id': _newId(),
          'check_fingerprint': checkFingerprint,
          'check_mbid': checkMbid,
          'check_file_size_duration': checkFileSizeDuration,
          'check_title_artist': checkTitleArtist,
          'min_confidence': ?minConfidence,
          'source_id': ?sourceId,
        }),
      );
      if (response.statusCode != 200) return null;
      final data = jsonDecode(response.body) as Map<String, dynamic>;
      final groupsJson = data['groups'] as List<dynamic>? ?? [];
      return FindDuplicatesResponse(
        id: '',
        groups: groupsJson
            .map(
              (g) => DuplicateGroup(
                method: g['method'] as String,
                key: g['key'] as String,
                confidence: (g['confidence'] as num).toDouble(),
                tracks: (g['tracks'] as List<dynamic>)
                    .map(
                      (t) => DuplicateTrackEntry(
                        trackId: t['track_id'] as String,
                        title: t['title'] as String,
                        artist: t['artist'] as String,
                        album: t['album'] as String,
                        filePath: t['file_path'] as String,
                        fileSize: t['file_size'] as int?,
                        durationSecs: (t['duration_secs'] as num).toDouble(),
                        mbidRecording: t['mbid_recording'] as String?,
                        hasFingerprint: t['has_fingerprint'] as bool,
                      ),
                    )
                    .toList(),
              ),
            )
            .toList(),
        totalDuplicates: (data['total_duplicates'] as num).toInt(),
        totalGroups: (data['total_groups'] as num).toInt(),
        error: data['error'] as String?,
      );
    } catch (e) {
      log('findDuplicates error: $e', isError: true);
      return null;
    }
  }

  // ---------------------------------------------------------------------------
  // ListenBrainz: Update Now Playing
  // ---------------------------------------------------------------------------

  Future<bool> updateNowPlaying(String trackId) async {
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/discovery/lb/now-playing'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({'id': _newId(), 'track_id': trackId}),
      );
      return response.statusCode == 200;
    } catch (e) {
      log('updateNowPlaying error: $e', isError: true);
      return false;
    }
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
    try {
      final isPlaylist = switch (recType) {
        'weekly' || 'daily' || 'weekly-exploration' || 'year' => true,
        _ => false,
      };
      final endpoint = isPlaylist ? '/playlist' : '/recommendations';
      final params = <String, String>{};
      params['rec_type'] = recType;
      if (isPlaylist && index != null) params['index'] = index.toString();
      if (!isPlaylist) {
        if (count != null) params['count'] = count.toString();
        if (offset != null) params['offset'] = offset.toString();
      }
      final uri = Uri.parse(
        '$baseUrl/api/tawai/discovery/lb$endpoint',
      ).replace(queryParameters: params);
      final response = await http.get(uri, headers: _authHeaders());
      if (response.statusCode == 200) {
        final obj = jsonDecode(response.body) as Map<String, dynamic>;
        final list = obj['recommendations'] as List<dynamic>;
        return (
          recordings: _parseDiscoveryList(list),
          playlistTitle: obj['playlist_title'] as String?,
          playlistId: obj['playlist_id'] as String?,
          playlistCount: obj['playlist_count'] as int?,
        );
      }
    } catch (e) {
      log('getLBRecommendations error: $e', isError: true);
    }
    return (
      recordings: <DiscoveryRecording>[],
      playlistTitle: null,
      playlistId: null,
      playlistCount: null,
    );
  }

  List<DiscoveryRecording> _parseDiscoveryList(List<dynamic> list) {
    return list.map((e) {
      final m = e as Map<String, dynamic>;
      return DiscoveryRecording(
        id: m['id'] as String,
        title: m['title'] as String? ?? '',
        artist: m['artist'] as String? ?? '',
        artistId: m['artist_id'] as String?,
        durationSecs: (m['duration_secs'] as num?)?.toDouble(),
        albumTitle: m['album_title'] as String?,
        cover: m['cover'] as String?,
        score: (m['score'] as num?)?.toDouble() ?? 0,
        isOwned: m['is_owned'] as bool? ?? false,
      );
    }).toList();
  }

  // ---------------------------------------------------------------------------
  // ListenBrainz: Validate Token
  // ---------------------------------------------------------------------------

  Future<({bool valid, String? userName, String message})> validateLBToken(
    String token,
  ) async {
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/discovery/lb/validate'),
        headers: {'Content-Type': 'application/json'},
        body: jsonEncode({'id': _newId(), 'token': token}),
      );
      if (response.statusCode == 200) {
        final data = jsonDecode(response.body) as Map<String, dynamic>;
        return (
          valid: data['valid'] as bool,
          userName: data['user_name'] as String?,
          message: data['message'] as String? ?? '',
        );
      }
    } catch (e) {
      log('validateLBToken error: $e', isError: true);
    }
    return (valid: false, userName: null, message: 'Request failed');
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
    try {
      final userId = SettingsManager.currentUserId.value ?? '';
      final response = await http.post(
        Uri.parse('$baseUrl/api/tawai/discovery/sync-recs'),
        headers: _authHeaders(extra: {'Content-Type': 'application/json'}),
        body: jsonEncode({
          'id': _newId(),
          'user_id': userId,
          'included_keys': includedKeys,
        }),
      );
      if (response.statusCode == 200) {
        final data = jsonDecode(response.body) as Map<String, dynamic>;
        return (
          success: data['success'] as bool,
          addedSources: List<String>.from(data['added_sources'] as List),
          removedSources: List<String>.from(data['removed_sources'] as List),
          tracksAdded: (data['tracks_added'] as num).toInt(),
          tracksRemoved: (data['tracks_removed'] as num).toInt(),
          error: data['error'] as String?,
        );
      }
    } catch (e) {
      log('syncRecs error: $e', isError: true);
    }
    return (
      success: false,
      addedSources: <String>[],
      removedSources: <String>[],
      tracksAdded: 0,
      tracksRemoved: 0,
      error: 'Request failed' as String?,
    );
  }

  void clearAuth() {
    _jwt = '';
    _csrfToken = '';
  }

  Future<void> logout() async {
    if (kIsWeb) {
      try {
        await http.post(
          Uri.parse('$baseUrl/api/tawai/auth/logout'),
          headers: _authHeaders(),
        );
      } catch (e) {
        log('Logout request failed: $e', isError: true);
      }
    }
    clearAuth();
  }

  void dispose() {
    _timer?.cancel();
    SettingsManager.serverHost.removeListener(restartPolling);
    SettingsManager.serverPort.removeListener(restartPolling);
  }
}
