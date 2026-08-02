class LyricsLine {
  final Duration? timestamp;
  final String text;

  const LyricsLine({this.timestamp, required this.text});
}

class ParsedLyrics {
  final List<LyricsLine> lines;
  final bool synced;
  final bool instrumental;

  const ParsedLyrics({
    required this.lines,
    required this.synced,
    this.instrumental = false,
  });

  factory ParsedLyrics.fromRaw(
    String raw, {
    bool synced = false,
    bool instrumental = false,
  }) {
    if (raw.trim().isEmpty) {
      return ParsedLyrics(lines: [], synced: false, instrumental: instrumental);
    }

    if (synced) {
      return _parseLrc(raw, instrumental: instrumental);
    }

    final lines = raw
        .split('\n')
        .map((line) => LyricsLine(text: line.trimRight()))
        .toList();
    return ParsedLyrics(lines: lines, synced: false, instrumental: instrumental);
  }

  static ParsedLyrics _parseLrc(String raw, {bool instrumental = false}) {
    final lines = <LyricsLine>[];
    final lineRegex =
        RegExp(r'\[(\d{2}):(\d{2})(?:\.(\d{2,3}))?\](.*)');
    final metaRegex = RegExp(r'^\[(ti|ar|al|by|offset|re|ve):.*\]$');

    for (final line in raw.split('\n')) {
      final trimmed = line.trim();
      if (trimmed.isEmpty) continue;

      if (metaRegex.hasMatch(trimmed)) continue;

      final matches = lineRegex.allMatches(trimmed);
      if (matches.isEmpty) {
        lines.add(LyricsLine(text: trimmed));
        continue;
      }

      String text = '';
      for (final m in matches) {
        text = (m.group(4) ?? '').trim();
        final minutes = int.parse(m.group(1)!);
        final seconds = int.parse(m.group(2)!);
        double ms = 0;
        if (m.group(3) != null) {
          final frac = m.group(3)!;
          ms = frac.length == 2 ? int.parse(frac) * 10.0 : int.parse(frac).toDouble();
        }
        lines.add(LyricsLine(
          timestamp: Duration(
            minutes: minutes,
            seconds: seconds,
            milliseconds: ms.round(),
          ),
          text: text,
        ));
      }
    }

    return ParsedLyrics(lines: lines, synced: true, instrumental: instrumental);
  }

  static bool looksSynced(String text) {
    return RegExp(r'\[\d{2}:\d{2}(?:\.\d{2,3})?\]').hasMatch(text);
  }

  static int currentLineIndex(Duration position, List<LyricsLine> lines) {
    final timed = <int>[];
    for (var i = 0; i < lines.length; i++) {
      if (lines[i].timestamp != null) timed.add(i);
    }
    if (timed.isEmpty) return -1;

    int lo = 0, hi = timed.length - 1, result = -1;
    while (lo <= hi) {
      final mid = (lo + hi) ~/ 2;
      if (lines[timed[mid]].timestamp! <= position) {
        result = timed[mid];
        lo = mid + 1;
      } else {
        hi = mid - 1;
      }
    }
    return result;
  }
}
