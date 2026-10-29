import 'package:flutter_test/flutter_test.dart';
import 'package:tawai/utils/lyrics_parser.dart';

void main() {
  ParsedLyrics parse(String raw, {Duration? trackDuration}) =>
      ParsedLyrics.fromRaw(raw, trackDuration: trackDuration);

  void expectTimestamp(
    String raw,
    Duration expected, {
    String? text,
    Duration? trackDuration,
  }) {
    final parsed = parse(raw, trackDuration: trackDuration);
    expect(parsed.synced, isTrue, reason: 'not parsed as synced: $raw');
    expect(parsed.lines, isNotEmpty);
    expect(parsed.lines.first.timestamp, expected, reason: raw);
    if (text != null) {
      expect(parsed.lines.first.text, text, reason: raw);
    }
  }

  group('standard timestamps', () {
    test('single digit minutes', () {
      expectTimestamp(
        '[0:00.40]今天',
        const Duration(milliseconds: 400),
        text: '今天',
      );
    });

    test('two digit minutes', () {
      expectTimestamp(
        '[00:00.40]Hello',
        const Duration(milliseconds: 400),
        text: 'Hello',
      );
    });

    test('one to three fractional digits', () {
      expectTimestamp('[0:00.4]a', const Duration(milliseconds: 400));
      expectTimestamp('[0:00.40]a', const Duration(milliseconds: 400));
      expectTimestamp('[0:00.400]a', const Duration(milliseconds: 400));
    });

    test('comma fraction', () {
      expectTimestamp('[0:00,40]a', const Duration(milliseconds: 400));
    });

    test('no fraction', () {
      expectTimestamp('[0:05]Intro', const Duration(seconds: 5), text: 'Intro');
    });

    test('long minutes', () {
      expectTimestamp(
        '[123:45.67]a',
        const Duration(minutes: 123, seconds: 45, milliseconds: 670),
      );
    });

    test('missing space after bracket', () {
      expectTimestamp(
        '[0:00.40]NoSpace',
        const Duration(milliseconds: 400),
        text: 'NoSpace',
      );
    });

    test('trailing whitespace is trimmed', () {
      expectTimestamp(
        '[0:00.40]Padded   ',
        const Duration(milliseconds: 400),
        text: 'Padded',
      );
    });

    test('no space after colon in time', () {
      expectTimestamp(
        '[1:02.80](No detours, no detours) ',
        const Duration(minutes: 1, seconds: 2, milliseconds: 800),
        text: '(No detours, no detours)',
      );
    });
  });

  group('legacy colon fraction', () {
    test('defaults to centiseconds', () {
      expectTimestamp('[0:00:40]Hello', const Duration(milliseconds: 400));
      expectTimestamp('[0:01:30]Hello', const Duration(milliseconds: 1300));
    });
  });

  group('hour timestamps', () {
    test('hours with dot fraction', () {
      expectTimestamp(
        '[0:01:30.40]Hello',
        const Duration(minutes: 1, seconds: 30, milliseconds: 400),
      );
    });

    test('hours with comma fraction', () {
      expectTimestamp(
        '[0:01:30,40]Hello',
        const Duration(minutes: 1, seconds: 30, milliseconds: 400),
      );
    });

    test('hours with millisecond fraction', () {
      expectTimestamp(
        '[1:02:03.500]Hello',
        const Duration(hours: 1, minutes: 2, seconds: 3, milliseconds: 500),
      );
    });

    test('four field hours with colon fraction', () {
      expectTimestamp(
        '[0:01:30:400]Hello',
        const Duration(minutes: 1, seconds: 30, milliseconds: 400),
      );
    });

    test('whitespace inside brackets', () {
      expectTimestamp(
        '[ 0:01:30.40 ] Hello',
        const Duration(minutes: 1, seconds: 30, milliseconds: 400),
        text: 'Hello',
      );
    });

    test('explicit hours form makes bare three field lines hours', () {
      final parsed = parse('[0:01:30.40]first\n[0:02:00]second');
      expect(
        parsed.lines[0].timestamp,
        const Duration(minutes: 1, seconds: 30, milliseconds: 400),
      );
      expect(parsed.lines[1].timestamp, const Duration(minutes: 2));
      expect(parsed.lines[1].text, 'second');
    });
  });

  group('ambiguous three field timestamps', () {
    test('uses track duration to choose hours', () {
      expectTimestamp(
        '[0:01:30]Hello',
        const Duration(minutes: 1, seconds: 30),
        trackDuration: const Duration(minutes: 3),
      );
    });

    test('uses track duration to reject hours', () {
      expectTimestamp(
        '[2:30:00]Hello',
        const Duration(minutes: 2, seconds: 30),
        trackDuration: const Duration(minutes: 3),
      );
    });

    test('falls back to legacy without track duration', () {
      expectTimestamp('[0:01:30]Hello', const Duration(milliseconds: 1300));
    });

    test('bracketed text is not mistaken for an hours timestamp', () {
      final parsed = parse('[0:01:30]first\n[note:do:re:mi]second');
      expect(parsed.lines[0].timestamp, const Duration(milliseconds: 1300));
      expect(parsed.lines[1].text, '[note:do:re:mi]second');
    });
  });

  group('line structure', () {
    test('multiple timestamps on one line', () {
      final parsed = parse('[00:10.00][00:30.00]Chorus');
      expect(parsed.synced, isTrue);
      expect(parsed.lines, hasLength(2));
      expect(parsed.lines[0].timestamp, const Duration(seconds: 10));
      expect(parsed.lines[1].timestamp, const Duration(seconds: 30));
      expect(parsed.lines[0].text, 'Chorus');
      expect(parsed.lines[1].text, 'Chorus');
    });

    test('strips enhanced LRC word timestamps', () {
      final parsed = parse('[00:12.00]<00:12.00>Hello <00:13.00>world');
      expect(parsed.lines.single.timestamp, const Duration(seconds: 12));
      expect(parsed.lines.single.text, 'Hello world');
    });

    test('strips hour-form word timestamps', () {
      final parsed = parse('[0:01:30.40]<0:01:30.40>Hello');
      expect(
        parsed.lines.single.timestamp,
        const Duration(minutes: 1, seconds: 30, milliseconds: 400),
      );
      expect(parsed.lines.single.text, 'Hello');
    });

    test('strips a leading BOM', () {
      final parsed = parse('\uFEFF[0:00.40]First\n[0:00.50]Second');
      expect(parsed.lines[0].timestamp, const Duration(milliseconds: 400));
      expect(parsed.lines[1].timestamp, const Duration(milliseconds: 500));
    });

    test('skips metadata tags', () {
      final parsed = parse(
        '[ti:Song]\n[ar:Artist]\n[offset:500]\n[length:03:12]\n'
        '[0:00.40]Hello',
      );
      expect(parsed.lines, hasLength(1));
      expect(parsed.lines.single.text, 'Hello');
    });

    test('plain lyrics stay plain', () {
      final parsed = parse('Just a line\nAnother line');
      expect(parsed.synced, isFalse);
      expect(parsed.lines.map((line) => line.text), [
        'Just a line',
        'Another line',
      ]);
    });

    test('bracketed non-timestamps stay plain', () {
      final parsed = parse('[Chorus]\nLa la la');
      expect(parsed.synced, isFalse);
      expect(parsed.lines.map((line) => line.text), ['[Chorus]', 'La la la']);
    });

    test('synced flag on plain text does not fake synced lyrics', () {
      final parsed = ParsedLyrics.fromRaw('No timestamps here', synced: true);
      expect(parsed.synced, isFalse);
      expect(parsed.lines.single.timestamp, isNull);
    });
  });

  group('looksSynced', () {
    test('accepts every supported timestamp form', () {
      for (final sample in [
        '[0:00.40]a',
        '[00:00.40]a',
        '[0:00.4]a',
        '[0:00,40]a',
        '[0:05]a',
        '[0:00:40]a',
        '[0:01:30.40]a',
        '[0:01:30,40]a',
        '[1:02:03.500]a',
        '[0:01:30:400]a',
        '[123:45.67]a',
      ]) {
        expect(ParsedLyrics.looksSynced(sample), isTrue, reason: sample);
      }
    });

    test('rejects plain lyrics and metadata', () {
      for (final sample in ['Just a line', '[Chorus]', '[ti:Song]', '']) {
        expect(ParsedLyrics.looksSynced(sample), isFalse, reason: sample);
      }
    });
  });

  group('currentLineIndex', () {
    test('finds the active line', () {
      final parsed = parse('[0:00.00]a\n[0:10.00]b\n[0:20.00]c');
      expect(
        ParsedLyrics.currentLineIndex(
          const Duration(seconds: 15),
          parsed.lines,
        ),
        1,
      );
      expect(
        ParsedLyrics.currentLineIndex(
          const Duration(seconds: 25),
          parsed.lines,
        ),
        2,
      );
      expect(ParsedLyrics.currentLineIndex(Duration.zero, parsed.lines), 0);
      expect(
        ParsedLyrics.currentLineIndex(
          const Duration(seconds: -1),
          parsed.lines,
        ),
        -1,
      );
    });
  });
}
