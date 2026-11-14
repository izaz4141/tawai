import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:tawai/models/stream_info.dart';
import 'package:tawai/ui/pages/player/widgets/stream_info_bar.dart';

StreamInfo _info({
  StreamMode mode = StreamMode.local,
  String? codec,
  int? bitrateKbps,
  int? sampleRateHz,
  int? channels,
  int? bitDepth,
  String? transport,
  bool loading = false,
  String? error,
}) {
  return StreamInfo(
    mode: mode,
    codec: codec,
    bitrateKbps: bitrateKbps,
    sampleRateHz: sampleRateHz,
    channels: channels,
    bitDepth: bitDepth,
    transport: transport,
    loading: loading,
    error: error,
  );
}

Widget _wrap(Widget child) {
  return MaterialApp(
    home: Scaffold(body: Center(child: child)),
  );
}

void main() {
  group('StreamInfo.summary', () {
    test('shows only detected values', () {
      final info = _info(
        mode: StreamMode.direct,
        codec: 'flac',
        bitrateKbps: 1411,
        sampleRateHz: 44100,
        bitDepth: 16,
        channels: 2,
      );

      expect(info.summary, 'Direct · FLAC · 1411 kbps · 44.1 kHz');
    });

    test('never invents missing fields', () {
      expect(_info(mode: StreamMode.local).summary, 'Local');
      expect(
        _info(mode: StreamMode.direct, codec: 'mp3').summary,
        'Direct · MP3',
      );
    });

    test('shows analyzing while probing', () {
      expect(
        StreamInfo.probing(StreamMode.transcode).summary,
        'Transcode · Analyzing…',
      );
    });
  });

  group('StreamInfo.detailRows', () {
    test('includes every detected property', () {
      final rows = _info(
        mode: StreamMode.transcode,
        codec: 'aac',
        bitrateKbps: 192,
        sampleRateHz: 48000,
        bitDepth: 24,
        channels: 2,
        transport: 'dash',
      ).detailRows;

      final values = {for (final row in rows) row.label: row.value};
      expect(values['Mode'], 'Transcode');
      expect(values['Codec'], 'aac');
      expect(values['Bitrate (average)'], '192 kbps');
      expect(values['Sample rate'], '48 kHz (48000 Hz)');
      expect(values['Bit depth'], '24-bit');
      expect(values['Channels'], '2 (stereo)');
      expect(values['Source'], 'DASH segment');
    });

    test('states when details are unavailable', () {
      final rows = _info(mode: StreamMode.direct).detailRows;

      expect(rows.map((row) => row.label), contains('Details'));
      expect(
        rows.firstWhere((row) => row.label == 'Details').value,
        'Unavailable',
      );
    });

    test('loading marks the row as analyzing', () {
      final rows = StreamInfo.probing(StreamMode.preview).detailRows;

      expect(
        rows.firstWhere((row) => row.label == 'Status').value,
        'Analyzing…',
      );
    });
  });

  group('StreamProbeResult', () {
    test('parses every server mode', () {
      expect(StreamProbeResult.parseMode('local'), StreamMode.local);
      expect(StreamProbeResult.parseMode('direct'), StreamMode.direct);
      expect(StreamProbeResult.parseMode('transcode'), StreamMode.transcode);
      expect(StreamProbeResult.parseMode('preview'), StreamMode.preview);
      expect(StreamProbeResult.parseMode('external'), StreamMode.external);
      expect(StreamProbeResult.parseMode('nonsense'), StreamMode.unknown);
      expect(StreamProbeResult.parseMode(null), StreamMode.unknown);
    });

    test('keeps the fallback mode when the probe omitted one', () {
      const result = StreamProbeResult(codec: 'opus');

      final info = result.toStreamInfo(fallbackMode: StreamMode.transcode);

      expect(info.mode, StreamMode.transcode);
      expect(info.codec, 'opus');
    });
  });

  group('StreamInfoBar', () {
    testWidgets('renders nothing without stream info', (tester) async {
      final notifier = ValueNotifier<StreamInfo?>(null);
      addTearDown(notifier.dispose);

      await tester.pumpWidget(_wrap(StreamInfoBar(info: notifier)));

      expect(find.textContaining('Local'), findsNothing);
      expect(find.byIcon(Icons.graphic_eq), findsNothing);
    });

    testWidgets('shows a spinner while probing', (tester) async {
      final notifier = ValueNotifier<StreamInfo?>(
        StreamInfo.probing(StreamMode.direct),
      );
      addTearDown(notifier.dispose);

      await tester.pumpWidget(_wrap(StreamInfoBar(info: notifier)));

      expect(find.text('Direct · Analyzing…'), findsOneWidget);
      expect(find.byType(CircularProgressIndicator), findsOneWidget);
    });

    testWidgets('updates when the probe finishes', (tester) async {
      final notifier = ValueNotifier<StreamInfo?>(
        StreamInfo.probing(StreamMode.direct),
      );
      addTearDown(notifier.dispose);

      await tester.pumpWidget(_wrap(StreamInfoBar(info: notifier)));
      notifier.value = _info(
        mode: StreamMode.direct,
        codec: 'flac',
        bitrateKbps: 1411,
        sampleRateHz: 44100,
      );
      await tester.pump();

      expect(find.text('Direct · FLAC · 1411 kbps · 44.1 kHz'), findsOneWidget);
      expect(find.byType(CircularProgressIndicator), findsNothing);
    });

    testWidgets('sheet reflects probe completion while open', (tester) async {
      final notifier = ValueNotifier<StreamInfo?>(
        StreamInfo.probing(StreamMode.direct),
      );
      addTearDown(notifier.dispose);

      await tester.pumpWidget(_wrap(StreamInfoBar(info: notifier)));
      await tester.tap(find.text('Direct · Analyzing…'));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 500));

      expect(find.text('Stream details'), findsOneWidget);
      expect(find.text('Status'), findsOneWidget);
      expect(find.text('Analyzing…'), findsOneWidget);

      notifier.value = _info(
        mode: StreamMode.direct,
        codec: 'flac',
        bitrateKbps: 1411,
        sampleRateHz: 44100,
      );
      await tester.pump();
      await tester.pumpAndSettle();

      expect(find.text('Codec'), findsOneWidget);
      expect(find.text('flac'), findsOneWidget);
      expect(find.text('Bitrate (average)'), findsOneWidget);
      expect(find.text('1411 kbps'), findsOneWidget);
      expect(find.text('44.1 kHz (44100 Hz)'), findsOneWidget);
      expect(find.text('Analyzing…'), findsNothing);
    });

    testWidgets('opens a detail sheet with only detected values', (
      tester,
    ) async {
      final notifier = ValueNotifier<StreamInfo?>(
        _info(mode: StreamMode.direct, codec: 'mp3', bitrateKbps: 320),
      );
      addTearDown(notifier.dispose);

      await tester.pumpWidget(_wrap(StreamInfoBar(info: notifier)));
      await tester.tap(find.text('Direct · MP3 · 320 kbps'));
      await tester.pumpAndSettle();

      expect(find.text('Stream details'), findsOneWidget);
      expect(find.text('Mode'), findsOneWidget);
      expect(find.text('Codec'), findsOneWidget);
      expect(find.text('Bitrate (average)'), findsOneWidget);
      expect(find.text('Sample rate'), findsNothing);
    });

    testWidgets('explains when the probe could not inspect the stream', (
      tester,
    ) async {
      final notifier = ValueNotifier<StreamInfo?>(
        _info(mode: StreamMode.transcode, error: 'ffprobe failed'),
      );
      addTearDown(notifier.dispose);

      await tester.pumpWidget(_wrap(StreamInfoBar(info: notifier)));
      await tester.tap(find.text('Transcode'));
      await tester.pumpAndSettle();

      expect(find.textContaining('could not be inspected'), findsOneWidget);
    });
  });
}
