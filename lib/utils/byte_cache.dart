import 'dart:collection';
import 'dart:typed_data';

class ByteCache {
  ByteCache({this.maxEntries = 256});

  final int maxEntries;

  final LinkedHashMap<String, Uint8List> _cache =
      LinkedHashMap<String, Uint8List>();
  final Map<String, Future<Uint8List?>> _inflight = {};

  Uint8List? peek(String key) {
    final cached = _cache.remove(key);
    if (cached == null) return null;
    _cache[key] = cached;
    return cached;
  }

  Future<Uint8List?> get(String key, Future<Uint8List?> Function() loader) {
    final cached = peek(key);
    if (cached != null) return Future.value(cached);

    final pending = _inflight[key];
    if (pending != null) return pending;

    final future = _load(key, loader);
    _inflight[key] = future;
    future.whenComplete(() => _inflight.remove(key));
    return future;
  }

  Future<Uint8List?> _load(
    String key,
    Future<Uint8List?> Function() loader,
  ) async {
    final bytes = await loader();
    if (bytes != null) {
      _cache[key] = bytes;
      _evictIfNeeded();
    }
    return bytes;
  }

  void invalidate(String key) {
    _cache.remove(key);
  }

  void clear() {
    _cache.clear();
    _inflight.clear();
  }

  void _evictIfNeeded() {
    while (_cache.length > maxEntries) {
      _cache.remove(_cache.keys.first);
    }
  }
}
