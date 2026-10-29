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
    Duration? trackDuration,
  }) {
    final source = _stripBom(raw);
    if (source.trim().isEmpty) {
      return ParsedLyrics(lines: [], synced: false, instrumental: instrumental);
    }

    if (synced || looksSynced(source)) {
      return _parseLrc(
        source,
        instrumental: instrumental,
        trackDuration: trackDuration,
      );
    }

    final lines = source
        .split('\n')
        .map((line) => LyricsLine(text: line.trimRight()))
        .toList();
    return ParsedLyrics(
      lines: lines,
      synced: false,
      instrumental: instrumental,
    );
  }

  // Leading `[...]` token. Used for both detection and parsing so the two
  // can never disagree about what counts as a timestamp.
  static final RegExp _bracketRe = RegExp(r'^\[([^\]]*)\]');

  // Fractional part after the last `.` or `,`, e.g. `.40` in `0:00.40`.
  static final RegExp _fractionRe = RegExp(r'[.,](\d{1,3})$');

  // Metadata tags such as `[ti:...]`, `[ar:...]`, `[offset:...]`, `[length:...]`.
  static final RegExp _metaRe = RegExp(r'^\[[A-Za-z][^\]]*:.*\]$');

  // Enhanced LRC word-level timestamps, e.g. `<00:12.00>`.
  static final RegExp _inlineTimestampRe = RegExp(
    r'<\d{1,3}(?::\d{1,3}){1,3}(?:[.,]\d{1,3})?>',
  );

  static String _stripBom(String text) =>
      text.startsWith('\uFEFF') ? text.substring(1) : text;

  static ParsedLyrics _parseLrc(
    String raw, {
    bool instrumental = false,
    Duration? trackDuration,
  }) {
    final rawLines = raw.split('\n');
    final assumeHours = _resolveAssumeHours(
      _collectTimestampTokens(rawLines),
      trackDuration,
    );

    final lines = <LyricsLine>[];

    for (final rawLine in rawLines) {
      final line = rawLine.trim();
      if (line.isEmpty) continue;

      var rest = line;
      final timestamps = <Duration>[];
      while (true) {
        final match = _bracketRe.firstMatch(rest);
        if (match == null) break;
        final timestamp = _parseTimestamp(
          match.group(1)!,
          assumeHours: assumeHours,
        );
        if (timestamp == null) break;
        timestamps.add(timestamp);
        rest = rest.substring(match.end);
      }

      if (timestamps.isEmpty) {
        if (_metaRe.hasMatch(line)) continue;
        lines.add(LyricsLine(text: line));
        continue;
      }

      final text = rest.replaceAll(_inlineTimestampRe, '').trim();
      for (final timestamp in timestamps) {
        lines.add(LyricsLine(timestamp: timestamp, text: text));
      }
    }

    return ParsedLyrics(
      lines: lines,
      synced: lines.any((line) => line.timestamp != null),
      instrumental: instrumental,
    );
  }

  /// Every leading bracket token in the file, timestamps and metadata alike.
  static List<String> _collectTimestampTokens(List<String> rawLines) {
    final tokens = <String>[];
    for (final rawLine in rawLines) {
      var rest = rawLine.trim();
      while (true) {
        final match = _bracketRe.firstMatch(rest);
        if (match == null) break;
        tokens.add(match.group(1)!);
        rest = rest.substring(match.end);
      }
    }
    return tokens;
  }

  /// Three colon-separated fields without a fraction are ambiguous:
  /// `[a:b:c]` can be `hh:mm:ss` or the legacy `mm:ss:cs`. Resolve it per file.
  static bool _resolveAssumeHours(
    List<String> tokens,
    Duration? trackDuration,
  ) {
    final ambiguous = <String>[];

    for (final token in tokens) {
      if (_isExplicitHours(token)) return true;
      final legacy = _parseTimestamp(token, assumeHours: false);
      final hours = _parseTimestamp(token, assumeHours: true);
      if (legacy != null && hours != null && legacy != hours) {
        ambiguous.add(token);
      }
    }

    if (ambiguous.isEmpty ||
        trackDuration == null ||
        trackDuration <= Duration.zero) {
      return false;
    }

    Duration maxFor({required bool assumeHours}) {
      var max = Duration.zero;
      for (final token in ambiguous) {
        final value = _parseTimestamp(token, assumeHours: assumeHours)!;
        if (value > max) max = value;
      }
      return max;
    }

    final legacyMax = maxFor(assumeHours: false);
    final hoursMax = maxFor(assumeHours: true);

    // Lyrics usually end near the end of the track, so the interpretation
    // whose last timestamp lands closest to the track duration is the one
    // the file almost certainly meant.
    const tolerance = Duration(seconds: 2);
    final limit = trackDuration + tolerance;

    final legacyFits = legacyMax <= limit;
    final hoursFits = hoursMax <= limit;

    if (legacyFits && hoursFits) {
      final legacyGap = (trackDuration - legacyMax).abs();
      final hoursGap = (trackDuration - hoursMax).abs();
      return hoursGap < legacyGap;
    }
    return hoursFits;
  }

  static bool _isExplicitHours(String inner) {
    // Must be a real timestamp first, otherwise `[a:b:c:d]`-style text would
    // flip the whole file into hours mode.
    if (_parseTimestamp(inner, assumeHours: false) == null) return false;

    final trimmed = inner.trim();
    final fraction = _fractionRe.firstMatch(trimmed);
    final head = fraction == null
        ? trimmed
        : trimmed.substring(0, fraction.start);
    final parts = head.split(':');
    return parts.length == 4 || (parts.length == 3 && fraction != null);
  }

  static Duration? _parseTimestamp(String inner, {required bool assumeHours}) {
    final trimmed = inner.trim();
    if (trimmed.isEmpty) return null;

    final fraction = _fractionRe.firstMatch(trimmed);
    final fractionDigits = fraction?.group(1);
    final head = fraction == null
        ? trimmed
        : trimmed.substring(0, fraction.start);
    final parts = head.split(':');

    if (parts.length < 2 || parts.length > 4) return null;
    if (parts.any((part) => part.isEmpty || int.tryParse(part) == null)) {
      return null;
    }

    if (parts.length == 2) {
      return Duration(
        minutes: int.parse(parts[0]),
        seconds: int.parse(parts[1]),
        milliseconds: _fractionToMs(fractionDigits),
      );
    }

    if (parts.length == 4) {
      return Duration(
        hours: int.parse(parts[0]),
        minutes: int.parse(parts[1]),
        seconds: int.parse(parts[2]),
        milliseconds: _fractionToMs(parts[3]),
      );
    }

    if (fractionDigits != null) {
      // `[h:mm:ss.ff]` can only mean hours.
      return Duration(
        hours: int.parse(parts[0]),
        minutes: int.parse(parts[1]),
        seconds: int.parse(parts[2]),
        milliseconds: _fractionToMs(fractionDigits),
      );
    }

    if (assumeHours) {
      return Duration(
        hours: int.parse(parts[0]),
        minutes: int.parse(parts[1]),
        seconds: int.parse(parts[2]),
      );
    }

    // Legacy `[mm:ss:cs]`.
    return Duration(
      minutes: int.parse(parts[0]),
      seconds: int.parse(parts[1]),
      milliseconds: _fractionToMs(parts[2]),
    );
  }

  static int _fractionToMs(String? digits) {
    if (digits == null || digits.isEmpty) return 0;
    final value = int.parse(digits);
    return switch (digits.length) {
      1 => value * 100,
      2 => value * 10,
      _ => value,
    };
  }

  static bool looksSynced(String text) {
    final source = _stripBom(text);
    for (final line in source.split('\n')) {
      final match = _bracketRe.firstMatch(line.trimLeft());
      if (match != null &&
          _parseTimestamp(match.group(1)!, assumeHours: false) != null) {
        return true;
      }
    }
    return false;
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
