import 'dart:typed_data';

import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;

import 'package:tawai/utils/api_service.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/byte_cache.dart';
import 'package:tawai/utils/logger.dart';

class AppImageCache {
  static final AppImageCache _instance = AppImageCache._();
  static AppImageCache get instance => _instance;
  AppImageCache._();

  final ByteCache _cache = ByteCache(maxEntries: 256);

  Uint8List? peekUrl(String url) => _cache.peek('url:$url');

  Future<Uint8List?> getUrl(String url) {
    return _cache.get('url:$url', () => _fetchUrl(url));
  }

  Future<Uint8List?> _fetchUrl(String url) async {
    try {
      final target = kIsWeb && !url.startsWith('data:')
          ? APIService.instance.wrapImageUrl(url)
          : url;
      final response = await http.get(Uri.parse(target));
      if (response.statusCode == 200 && response.bodyBytes.isNotEmpty) {
        return response.bodyBytes;
      }
    } catch (e) {
      log('getNetworkImage error for $url: $e', isError: true);
    }
    return null;
  }

  Uint8List? peek({String? albumId, String? trackId}) {
    final key = _coverKey(albumId: albumId, trackId: trackId);
    return key == null ? null : _cache.peek(key);
  }

  Future<Uint8List?> getCover({String? albumId, String? trackId}) {
    final key = _coverKey(albumId: albumId, trackId: trackId);
    if (key == null) return Future.value(null);
    return _cache.get(
      key,
      () => _fetchCover(albumId: albumId, trackId: trackId),
    );
  }

  Future<Uint8List?> _fetchCover({String? albumId, String? trackId}) async {
    if (albumId != null) {
      return BridgeService.instance.getAlbumCover(albumId);
    }
    if (trackId != null) {
      return BridgeService.instance.getTrackCover(trackId);
    }
    return null;
  }

  void invalidateUrl(String url) => _cache.invalidate('url:$url');

  void invalidate({String? albumId, String? trackId}) {
    final key = _coverKey(albumId: albumId, trackId: trackId);
    if (key != null) _cache.invalidate(key);
  }

  void clear() => _cache.clear();

  static String? _coverKey({String? albumId, String? trackId}) {
    if (albumId != null) return 'album:$albumId';
    if (trackId != null) return 'track:$trackId';
    return null;
  }
}
