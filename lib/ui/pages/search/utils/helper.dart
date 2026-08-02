import '../search_types.dart';

const _losslessExtensions = {'.flac', '.wav', '.alac', '.ape', '.wv', '.aiff'};

bool isLossless(String filename, int bitrate) {
  final dot = filename.lastIndexOf('.');
  if (dot >= 0) {
    final ext = filename.substring(dot).toLowerCase();
    if (_losslessExtensions.contains(ext)) return true;
  }
  if (bitrate >= 1411) return true;
  return false;
}

String? pickNadekodonFormat(
  List<Map<String, dynamic>> audioFormats,
  String quality,
) {
  var candidates = audioFormats.where((f) {
    final ext = (f['ext'] as String?)?.toLowerCase() ?? '';
    return ext == 'm4a' || ext == 'mp4';
  }).toList();
  if (candidates.isEmpty) candidates = audioFormats;
  if (candidates.isEmpty) return null;

  switch (quality) {
    case 'low':
      candidates.sort(
        (a, b) =>
            ((a['abr'] as num?) ?? 0).compareTo((b['abr'] as num?) ?? 0),
      );
      return candidates.first['format_id'] as String?;
    case 'high':
      var filtered = candidates
          .where((f) => ((f['abr'] as num?) ?? 0) >= 320)
          .toList();
      if (filtered.isNotEmpty) {
        filtered.sort(
          (a, b) =>
              ((a['abr'] as num?) ?? 0).compareTo((b['abr'] as num?) ?? 0),
        );
        return filtered.first['format_id'] as String?;
      }
      break;
    case 'medium':
      var filtered = candidates
          .where((f) => ((f['abr'] as num?) ?? 0) >= 192)
          .toList();
      if (filtered.isNotEmpty) {
        filtered.sort(
          (a, b) =>
              ((a['abr'] as num?) ?? 0).compareTo((b['abr'] as num?) ?? 0),
        );
        return filtered.first['format_id'] as String?;
      }
      break;
  }

  candidates.sort(
    (a, b) =>
        ((b['abr'] as num?) ?? 0).compareTo((a['abr'] as num?) ?? 0),
  );
  return candidates.first['format_id'] as String?;
}

SearchResultItem? pickBestMatch(
  List<SearchResultItem> entries,
  String quality,
) {
  final slskdEntries = entries
      .where((e) => e.sourceType == 'slskd')
      .toList();
  if (slskdEntries.isEmpty) return null;

  var candidates = List<SearchResultItem>.from(slskdEntries);

  switch (quality) {
    case 'lossless':
      candidates = candidates
          .where((e) => isLossless(e.filename, e.bitrate ?? 0))
          .toList();
      if (candidates.isEmpty) return null;
      candidates.sort(
        (a, b) => b.size.compareTo(a.size),
      );
      return candidates.first;
    case 'high':
      candidates = candidates
          .where(
            (e) =>
                isLossless(e.filename, e.bitrate ?? 0) ||
                (e.bitrate ?? 0) >= 320,
          )
          .toList();
      break;
    case 'medium':
      candidates = candidates
          .where(
            (e) =>
                isLossless(e.filename, e.bitrate ?? 0) ||
                (e.bitrate ?? 0) >= 192,
          )
          .toList();
      break;
    case 'low':
    case 'best':
      break;
  }

  if (candidates.isEmpty) return null;

  candidates.sort((a, b) {
    final aLossless = isLossless(a.filename, a.bitrate ?? 0);
    final bLossless = isLossless(b.filename, b.bitrate ?? 0);
    if (aLossless != bLossless) return bLossless ? 1 : -1;
    if (a.bitrate != b.bitrate) {
      return (b.bitrate ?? 0).compareTo(a.bitrate ?? 0);
    }
    return b.size.compareTo(a.size);
  });

  return candidates.first;
}
