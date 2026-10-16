import 'package:tawai/utils/helper.dart';

/// An audio format this app can play, and how the operating system should be
/// told about it.
class AudioFormat {
  /// Human readable name, e.g. `FLAC`.
  final String label;

  /// File extensions, lowercase and each prefixed with a dot.
  final List<String> extensions;

  /// MIME types to advertise in the desktop entry.
  ///
  /// Empty for formats the shared MIME database assigns to something other
  /// than audio: `*.mp4` belongs to `video/mp4`, and `.alac` is not registered
  /// at all. The desktop environment picks handlers by MIME type, so there is
  /// no way to claim those without also claiming every video, which is why they
  /// stay reachable only by extension, through the command line and through
  /// the Windows registry.
  final List<String> mimeTypes;

  const AudioFormat({
    required this.label,
    required this.extensions,
    this.mimeTypes = const [],
  });
}

/// Every audio format this app offers to open from outside the desktop
/// environment.
///
/// The MIME types come from the shared MIME database. Names like `audio/mp3` and
/// `audio/x-flac` are aliases the database keeps alongside the canonical type;
/// desktop environments match on an exact string, so both are listed. The one
/// exception is `audio/AMR`, which the database really does spell in capitals,
/// and which is therefore listed that way.
const List<AudioFormat> audioFormats = [
  AudioFormat(
    label: 'MP3',
    extensions: ['.mp3'],
    mimeTypes: ['audio/mpeg', 'audio/mp3', 'audio/x-mp3', 'audio/x-mpeg'],
  ),
  AudioFormat(
    label: 'AAC',
    extensions: ['.aac'],
    mimeTypes: ['audio/aac', 'audio/x-aac'],
  ),
  AudioFormat(
    label: 'MPEG-4 audio',
    extensions: ['.m4a'],
    mimeTypes: ['audio/mp4', 'audio/x-m4a', 'audio/m4a'],
  ),
  AudioFormat(
    label: 'MPEG-4 audiobook',
    extensions: ['.m4b'],
    mimeTypes: ['audio/x-m4b'],
  ),
  AudioFormat(label: 'MPEG-4', extensions: ['.mp4']),
  AudioFormat(label: 'Apple Lossless', extensions: ['.alac']),
  AudioFormat(
    label: 'FLAC',
    extensions: ['.flac'],
    mimeTypes: ['audio/flac', 'audio/x-flac'],
  ),
  AudioFormat(
    label: 'WAV',
    extensions: ['.wav'],
    mimeTypes: ['audio/vnd.wave', 'audio/wav', 'audio/x-wav'],
  ),
  AudioFormat(
    label: 'AIFF',
    extensions: ['.aif', '.aiff'],
    mimeTypes: ['audio/x-aiff'],
  ),
  AudioFormat(
    label: 'Ogg Vorbis',
    extensions: ['.ogg', '.oga'],
    mimeTypes: ['audio/ogg', 'audio/x-vorbis+ogg'],
  ),
  AudioFormat(
    label: 'Opus',
    extensions: ['.opus'],
    mimeTypes: ['audio/ogg', 'audio/x-opus+ogg'],
  ),
  AudioFormat(
    label: 'Speex',
    extensions: ['.spx'],
    mimeTypes: ['audio/x-speex'],
  ),
  AudioFormat(
    label: 'WMA',
    extensions: ['.wma'],
    mimeTypes: ['audio/x-ms-wma'],
  ),
  AudioFormat(label: 'AMR', extensions: ['.amr'], mimeTypes: ['audio/AMR']),
  AudioFormat(
    label: 'Monkey’s Audio',
    extensions: ['.ape'],
    mimeTypes: ['audio/x-ape'],
  ),
  AudioFormat(
    label: 'Musepack',
    extensions: ['.mpc'],
    mimeTypes: ['audio/x-musepack'],
  ),
  AudioFormat(
    label: 'WavPack',
    extensions: ['.wv'],
    mimeTypes: ['audio/x-wavpack'],
  ),
];

/// Every extension in [audioFormats], lowercase and dot prefixed.
final Set<String> audioExtensions = {
  for (final format in audioFormats) ...format.extensions,
};

/// Every MIME type in [audioFormats], in table order, for `MimeType=` in the
/// desktop entry.
///
/// A type shared by several formats, such as `audio/ogg` for both Ogg Vorbis and
/// Opus, is listed once.
final List<String> audioMimeTypes = [
  ...{for (final format in audioFormats) ...format.mimeTypes},
];

/// [audioExtensions] without the leading dot, for `file_picker`, which wants
/// bare extensions.
final List<String> audioBareExtensions = [
  for (final ext in audioExtensions) ext.substring(1),
];

/// Whether [path] looks like an audio file this app can open.
bool isAudioPath(String path) => audioExtensions.contains(extension(path));
