import 'dart:async';

import 'package:package_info_plus/package_info_plus.dart';
import 'package:tawai/models/audio_format.dart';
import 'package:tawai/utils/io_service.dart';

/// What the process was asked to do on the command line.
sealed class CliAction {
  const CliAction(this.io);

  final IOService io;

  /// Carries out the command. Returns normally if the app should keep starting;
  /// commands that print and exit never come back.
  Future<void> run() async {}
}

class ShowVersion extends CliAction {
  const ShowVersion(super.io);

  @override
  Future<void> run() async {
    final info = await PackageInfo.fromPlatform();
    io.writeLine('Tawai ${info.version}+${info.buildNumber}');
    io.exit(0);
  }
}

class ShowHelp extends CliAction {
  const ShowHelp(super.io);

  @override
  Future<void> run() async {
    io.writeLine(cliUsage);
    io.exit(0);
  }
}

/// Open the audio files the desktop environment handed us, either as local
/// paths or as `file://` URIs.
class OpenTarget extends CliAction {
  const OpenTarget(super.io, this.targets);

  final List<String> targets;
}

class RunNormally extends CliAction {
  const RunNormally(super.io);
}

const String cliUsage = '''
Tawai - a music player and metadata manager.

Usage:
  tawai [options] [file...]

Options:
  -v, --version   Print the version and exit
  -h, --help      Show this help and exit

''';

/// Interprets the command line the OS handed to this process. Only call this
/// where a real filesystem exists, since confirming a local path goes through
/// [IOService].
Future<CliAction> parseCliArgs(List<String> args, IOService io) async {
  for (final arg in args) {
    switch (arg) {
      case '--version':
      case '-v':
        return ShowVersion(io);
      case '--help':
      case '-h':
        return ShowHelp(io);
    }
  }

  // Flags win over targets, so only look for one once none were given.
  final targets = await _openTargets(args, io);
  if (targets.isNotEmpty) {
    return OpenTarget(io, targets);
  }

  return RunNormally(io);
}

Future<List<String>> _openTargets(List<String> args, IOService io) async {
  final targets = <String>[];
  for (final arg in args) {
    final target = _normalizeTarget(arg);
    if (target == null) continue;
    if (!isAudioPath(target)) continue;
    // Skipping a path that is not really there keeps a stale file association
    // from opening an error dialog.
    if (!await io.fileExists(target)) continue;
    targets.add(target);
  }
  return targets;
}

/// Turns a `file://` URI into a path, leaving anything else untouched.
String? _normalizeTarget(String arg) {
  if (arg.isEmpty) return null;

  if (arg.toLowerCase().startsWith('file://')) {
    try {
      return Uri.parse(arg).toFilePath();
    } on FormatException {
      return null;
    }
  }

  return arg;
}

/// Carries open targets from the OS into the widget tree.
///
/// A forwarded target can arrive before the UI is listening, while the primary
/// process is still starting up, so buffering here is what keeps it from being
/// dropped.
class OpenTargetService {
  OpenTargetService._();

  static final instance = OpenTargetService._();

  // Deliberately not broadcast: a single subscription stream buffers events
  // until `Tawai` listens, which is exactly the late subscriber case.
  final _controller = StreamController<List<String>>();

  /// Targets passed on the command line at process start.
  List<String>? _startupTargets;

  Stream<List<String>> get targets => _controller.stream;

  void setStartupTargets(List<String> targets) {
    if (targets.isEmpty) return;
    _startupTargets = targets;
  }

  /// Returns and clears the command line targets, if any.
  List<String>? takeStartupTargets() {
    final targets = _startupTargets;
    _startupTargets = null;
    return targets;
  }

  /// Records targets handed over by a second instance.
  void deliver(List<String> targets) {
    if (targets.isEmpty) return;
    _controller.add(targets);
  }
}
