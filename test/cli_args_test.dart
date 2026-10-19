import 'package:flutter_test/flutter_test.dart';

import 'package:tawai/utils/cli_args.dart';
import 'package:tawai/utils/io_service.dart';

/// Only [IOService.fileExists] matters to the parser, so everything else throws
/// rather than pretending to work.
class _FakeIOService implements IOService {
  _FakeIOService(this.existing);

  final Set<String> existing;

  @override
  Future<bool> fileExists(String path) async => existing.contains(path);

  @override
  dynamic noSuchMethod(Invocation invocation) =>
      throw UnsupportedError('${invocation.memberName} is not used by tests');
}

void main() {
  late _FakeIOService io;

  setUp(() {
    io = _FakeIOService({
      '/music/a.mp3',
      '/music/b.flac',
      '/music/c.mp3',
      '/with space/d.opus',
      '/music/clip.mp4',
      '/music/Loud.MP3',
      '/music/notes.txt',
    });
  });

  Future<CliAction> parse(List<String> args) => parseCliArgs(args, io);

  group('flags', () {
    test('--version prints the version', () async {
      expect(await parse(['--version']), isA<ShowVersion>());
    });

    test('-v prints the version', () async {
      expect(await parse(['-v']), isA<ShowVersion>());
    });

    test('--help prints the usage', () async {
      expect(await parse(['--help']), isA<ShowHelp>());
    });

    test('-h prints the usage', () async {
      expect(await parse(['-h']), isA<ShowHelp>());
    });

    test('flags win over targets', () async {
      expect(await parse(['/music/a.mp3', '--help']), isA<ShowHelp>());
    });
  });

  group('targets', () {
    test('no arguments runs normally', () async {
      expect(await parse([]), isA<RunNormally>());
    });

    test('a single existing audio file is opened', () async {
      final action = await parse(['/music/a.mp3']);
      expect(action, isA<OpenTarget>());
      expect((action as OpenTarget).targets, ['/music/a.mp3']);
    });

    test('several files are kept in the order given', () async {
      final action =
          await parse(['/music/c.mp3', '/music/b.flac', '/music/a.mp3'])
              as OpenTarget;
      expect(action.targets, ['/music/c.mp3', '/music/b.flac', '/music/a.mp3']);
    });

    test('a file:// URI becomes a path', () async {
      final action = await parse(['file:///music/b.flac']) as OpenTarget;
      expect(action.targets, ['/music/b.flac']);
    });

    test('spaces in paths survive', () async {
      final action = await parse(['/with space/d.opus']) as OpenTarget;
      expect(action.targets, ['/with space/d.opus']);
    });

    test('uppercase extensions are accepted', () async {
      final action = await parse(['/music/Loud.MP3']) as OpenTarget;
      expect(action.targets, ['/music/Loud.MP3']);
    });

    test('.mp4 opens even though it has no MIME type', () async {
      final action = await parse(['/music/clip.mp4']) as OpenTarget;
      expect(action.targets, ['/music/clip.mp4']);
    });

    test('unrelated arguments are skipped, audio still opens', () async {
      final action =
          await parse(['--enable-logging', '/music/a.mp3']) as OpenTarget;
      expect(action.targets, ['/music/a.mp3']);
    });

    test('a non audio file is ignored', () async {
      expect(await parse(['/music/notes.txt']), isA<RunNormally>());
    });

    test('a missing file is ignored', () async {
      expect(await parse(['/music/gone.mp3']), isA<RunNormally>());
    });

    test('a stale association with no target runs normally', () async {
      expect(await parse(['/gone/a.mp3', '/gone/b.flac']), isA<RunNormally>());
    });

    test('remote urls are ignored', () async {
      expect(await parse(['https://example.com/a.mp3']), isA<RunNormally>());
    });

    test('magnet links are ignored', () async {
      expect(await parse(['magnet:?xt=urn:btih:abc']), isA<RunNormally>());
    });

    test('the executable itself is ignored', () async {
      expect(await parse(['/opt/tawai/bin/tawai']), isA<RunNormally>());
    });
  });

  group('OpenTargetService', () {
    test('startup targets are taken once', () {
      final service = OpenTargetService.instance;
      expect(service.takeStartupTargets(), isNull);

      service.setStartupTargets(['/music/a.mp3']);
      expect(service.takeStartupTargets(), ['/music/a.mp3']);
      expect(service.takeStartupTargets(), isNull);
    });

    test('empty startup targets are not remembered', () {
      final service = OpenTargetService.instance;
      service.setStartupTargets([]);
      expect(service.takeStartupTargets(), isNull);
    });
  });
}
