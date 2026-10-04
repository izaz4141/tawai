import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:tawai/models/audio_format.dart';
import 'package:tawai/utils/helper.dart';

void main() {
  group('AudioFormat table', () {
    test('every format has at least one extension', () {
      for (final format in audioFormats) {
        expect(format.label, isNotEmpty, reason: 'a format needs a label');
        expect(format.extensions, isNotEmpty, reason: '${format.label} has none');
      }
    });

    test('extensions are lowercase, dot prefixed and unique', () {
      final seen = <String, String>{};
      for (final format in audioFormats) {
        for (final ext in format.extensions) {
          expect(ext, startsWith('.'), reason: '$ext in ${format.label}');
          expect(ext, ext.toLowerCase(), reason: '$ext in ${format.label}');
          expect(ext.length, greaterThan(1), reason: 'bare dot in ${format.label}');
          expect(seen.containsKey(ext), isFalse,
              reason: '$ext is claimed by both ${seen[ext]} and ${format.label}');
          seen[ext] = format.label;
        }
      }
    });

    test('every MIME type is an audio type', () {
      for (final mime in audioMimeTypes) {
        expect(mime, startsWith('audio/'),
            reason: '$mime would offer Tawai for non audio files');
      }
    });

    test('no duplicate MIME types', () {
      expect(audioMimeTypes.toSet().length, audioMimeTypes.length);
    });

    test('audioBareExtensions mirrors audioExtensions without the dot', () {
      expect(audioBareExtensions.toSet(),
          audioExtensions.map((e) => e.substring(1)).toSet());
    });
  });

  group('extension()', () {
    test('lowercases', () {
      expect(extension('Song.FLAC'), '.flac');
    });

    test('handles Windows separators', () {
      expect(extension(r'C:\Music\Song.FLAC'), '.flac');
    });

    test('a leading dot is a dotfile, not an extension', () {
      expect(extension('/home/me/.mp3'), '');
      expect(extension('.mp3'), '');
    });

    test('no extension', () {
      expect(extension('/music/track'), '');
      expect(extension(''), '');
    });

    test('takes the last dot', () {
      expect(extension('/music/live.2.flac'), '.flac');
    });
  });

  group('basename()', () {
    test('strips directories', () {
      expect(basename('/music/song.mp3'), 'song.mp3');
    });

    test('handles Windows separators', () {
      expect(basename(r'C:\Music\Song.FLAC'), 'Song.FLAC');
    });

    test('no directory', () {
      expect(basename('song.mp3'), 'song.mp3');
    });

    test('trailing separator yields empty', () {
      expect(basename('/music/'), '');
    });
  });

  group('isAudioPath()', () {
    test('accepts a known extension', () {
      expect(isAudioPath('/home/me/song.mp3'), isTrue);
      expect(isAudioPath(r'C:\Music\Song.FLAC'), isTrue);
    });

    test('rejects an unknown extension', () {
      expect(isAudioPath('/music/song.txt'), isFalse);
      expect(isAudioPath('/music/song'), isFalse);
    });

    test('rejects a dotfile that looks like an audio file', () {
      expect(isAudioPath('/home/me/.mp3'), isFalse);
    });

    test('.mp4 opens by extension even though it has no MIME type', () {
      final mp4 = audioFormats.firstWhere((f) => f.extensions.contains('.mp4'));
      expect(mp4.mimeTypes, isEmpty);
      expect(isAudioPath('/home/me/clip.mp4'), isTrue);
    });

    test('every declared extension is accepted', () {
      for (final ext in audioExtensions) {
        expect(isAudioPath('/music/track$ext'), isTrue, reason: ext);
      }
    });
  });

  group('linux/tawai.desktop', () {
    late String desktop;

    setUpAll(() {
      desktop = File('linux/tawai.desktop').readAsStringSync();
    });

    test('MimeType matches the model', () {
      final expected = '${audioMimeTypes.join(';')};';
      final line = desktop
          .split('\n')
          .firstWhere((l) => l.startsWith('MimeType='), orElse: () => '');
      expect(line, 'MimeType=$expected');
    });

    test('launches with %F so multiple files arrive in one invocation', () {
      final line = desktop
          .split('\n')
          .firstWhere((l) => l.startsWith('Exec='), orElse: () => '');
      expect(line, contains('tawai'));
      expect(line, contains('%F'));
    });
  });
}
