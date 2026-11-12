/// How a track is being played.
enum StreamMode { local, direct, transcode, preview, external, unknown }

extension StreamModeLabel on StreamMode {
  String get label => switch (this) {
    StreamMode.local => 'Local',
    StreamMode.direct => 'Direct',
    StreamMode.transcode => 'Transcode',
    StreamMode.preview => 'Preview',
    StreamMode.external => 'External',
    StreamMode.unknown => 'Stream',
  };
}

/// A single labelled row shown in the stream-info sheet.
class StreamInfoRow {
  final String label;
  final String value;

  const StreamInfoRow(this.label, this.value);
}

/// Raw result of probing a stream, before presentation.
class StreamProbeResult {
  final StreamMode mode;
  final String? transport;
  final String? container;
  final String? containerLong;
  final String? codec;
  final String? codecLong;
  final int? bitrateKbps;
  final int? sampleRateHz;
  final int? bitDepth;
  final int? channels;
  final double? durationSecs;
  final int? sizeBytes;
  final String? error;

  const StreamProbeResult({
    this.mode = StreamMode.unknown,
    this.transport,
    this.container,
    this.containerLong,
    this.codec,
    this.codecLong,
    this.bitrateKbps,
    this.sampleRateHz,
    this.bitDepth,
    this.channels,
    this.durationSecs,
    this.sizeBytes,
    this.error,
  });

  static StreamMode parseMode(String? raw) {
    return switch (raw) {
      'local' => StreamMode.local,
      'direct' => StreamMode.direct,
      'transcode' => StreamMode.transcode,
      'preview' => StreamMode.preview,
      'external' => StreamMode.external,
      _ => StreamMode.unknown,
    };
  }

  StreamInfo toStreamInfo({StreamMode? fallbackMode}) {
    return StreamInfo(
      mode: mode == StreamMode.unknown
          ? (fallbackMode ?? StreamMode.unknown)
          : mode,
      transport: transport,
      container: container,
      containerLong: containerLong,
      codec: codec,
      codecLong: codecLong,
      bitrateKbps: bitrateKbps,
      sampleRateHz: sampleRateHz,
      bitDepth: bitDepth,
      channels: channels,
      durationSecs: durationSecs,
      sizeBytes: sizeBytes,
      error: error,
    );
  }
}

/// Detected information about the currently playing stream.
class StreamInfo {
  final StreamMode mode;
  final String? transport;
  final String? container;
  final String? containerLong;
  final String? codec;
  final String? codecLong;
  final int? bitrateKbps;
  final int? sampleRateHz;
  final int? bitDepth;
  final int? channels;
  final double? durationSecs;
  final int? sizeBytes;
  final bool loading;
  final String? error;

  const StreamInfo({
    required this.mode,
    this.transport,
    this.container,
    this.containerLong,
    this.codec,
    this.codecLong,
    this.bitrateKbps,
    this.sampleRateHz,
    this.bitDepth,
    this.channels,
    this.durationSecs,
    this.sizeBytes,
    this.loading = false,
    this.error,
  });

  factory StreamInfo.probing(StreamMode mode) {
    return StreamInfo(mode: mode, loading: true);
  }

  bool get hasDetails =>
      container != null ||
      codec != null ||
      bitrateKbps != null ||
      sampleRateHz != null ||
      bitDepth != null ||
      channels != null;

  /// Compact line under the title/artist row. Only detected values are shown.
  String get summary {
    final parts = <String>[mode.label];
    if (loading) {
      parts.add('Analyzing…');
    } else {
      final codec = this.codec;
      if (codec != null && codec.isNotEmpty) parts.add(codec.toUpperCase());
      final bitrate = bitrateKbps;
      if (bitrate != null) parts.add('$bitrate kbps');
      final sampleRate = sampleRateHz;
      if (sampleRate != null) parts.add(_formatSampleRate(sampleRate));
    }
    return parts.join(' · ');
  }

  List<StreamInfoRow> get detailRows {
    final rows = <StreamInfoRow>[StreamInfoRow('Mode', mode.label)];

    if (container != null) {
      final long = containerLong;
      rows.add(
        StreamInfoRow(
          'Container',
          long != null && long.isNotEmpty ? '$container ($long)' : container!,
        ),
      );
    }
    if (codec != null) {
      final long = codecLong;
      rows.add(
        StreamInfoRow(
          'Codec',
          long != null && long.isNotEmpty ? '$codec — $long' : codec!,
        ),
      );
    }
    if (bitrateKbps != null) {
      rows.add(StreamInfoRow('Bitrate (average)', '$bitrateKbps kbps'));
    }
    if (sampleRateHz != null) {
      rows.add(
        StreamInfoRow(
          'Sample rate',
          '${_formatSampleRate(sampleRateHz!)} ($sampleRateHz Hz)',
        ),
      );
    }
    if (bitDepth != null) {
      rows.add(StreamInfoRow('Bit depth', '$bitDepth-bit'));
    }
    if (channels != null) {
      rows.add(StreamInfoRow('Channels', _formatChannels(channels!)));
    }
    if (durationSecs != null) {
      rows.add(StreamInfoRow('Duration', _formatDuration(durationSecs!)));
    }
    if (sizeBytes != null) {
      rows.add(StreamInfoRow('Source size', _formatBytes(sizeBytes!)));
    }
    if (transport != null) {
      rows.add(StreamInfoRow('Source', _formatTransport(transport!)));
    }
    if (loading) {
      rows.add(const StreamInfoRow('Status', 'Analyzing…'));
    } else if (!hasDetails) {
      rows.add(const StreamInfoRow('Details', 'Unavailable'));
    }

    return rows;
  }

  static String _formatSampleRate(int hz) {
    if (hz % 1000 == 0) return '${hz ~/ 1000} kHz';
    return '${(hz / 1000).toStringAsFixed(1)} kHz';
  }

  static String _formatChannels(int channels) {
    if (channels == 1) return '1 (mono)';
    if (channels == 2) return '2 (stereo)';
    return '$channels';
  }

  static String _formatDuration(double seconds) {
    final total = seconds.round();
    final hours = total ~/ 3600;
    final minutes = (total % 3600) ~/ 60;
    final secs = total % 60;
    if (hours > 0) {
      return '$hours:${minutes.toString().padLeft(2, '0')}:'
          '${secs.toString().padLeft(2, '0')}';
    }
    return '$minutes:${secs.toString().padLeft(2, '0')}';
  }

  static String _formatBytes(int bytes) {
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    var value = bytes.toDouble();
    var unit = 0;
    while (value >= 1024 && unit < units.length - 1) {
      value /= 1024;
      unit++;
    }
    final digits = unit == 0 ? 0 : 1;
    return '${value.toStringAsFixed(digits)} ${units[unit]}';
  }

  static String _formatTransport(String transport) {
    return switch (transport) {
      'file' => 'Local file',
      'http' => 'HTTP stream',
      'dash' => 'DASH segment',
      'preview' => 'Preview stream',
      _ => transport,
    };
  }
}
