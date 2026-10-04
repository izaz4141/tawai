import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:path_provider/path_provider.dart';

import 'package:tawai/utils/logger.dart';

class SingleInstance {
  static const String _lockFileName = '.instance_lock';
  static const String _focusCommand = 'focus';

  static File? _lockFile;

  /// Initializes the single instance mechanism.
  ///
  /// If another instance is already running, this method will forward
  /// [startupTargets] to it and then exit the current process.
  ///
  /// If this is the main instance, it will start listening for signals from
  /// subsequent instances.
  ///
  /// [onFocus] is the callback to execute when a focus signal is received.
  /// [onOpenTargets] is the callback to execute when a second instance hands
  /// over the audio files it was launched with.
  static Future<void> init(
    void Function() onFocus,
    void Function(List<String> targets) onOpenTargets, {
    List<String> startupTargets = const [],
  }) async {
    final appDocDir = await getApplicationSupportDirectory();
    _lockFile = File('${appDocDir.path}/$_lockFileName');

    bool isMainInstance = false;

    if (await _lockFile!.exists()) {
      try {
        final port = int.parse(await _lockFile!.readAsString());
        final socket = await Socket.connect(InternetAddress.loopbackIPv4, port);
        socket.write(_requestFor(startupTargets));
        await socket.flush();
        await socket.close();
        exit(0);
      } catch (e) {
        // Connection failed, likely a stale lock file.
        // We will take over as the main instance.
        isMainInstance = true;
      }
    } else {
      isMainInstance = true;
    }

    if (isMainInstance) {
      await _becomeMainInstance(onFocus, onOpenTargets);
    }
  }

  /// What a second instance sends over the socket. Bare `focus` when it has
  /// nothing to hand over, so the oldest possible message shape is unchanged.
  static String _requestFor(List<String> targets) {
    if (targets.isEmpty) return _focusCommand;
    return jsonEncode({'cmd': 'open', 'paths': targets});
  }

  static Future<void> _becomeMainInstance(
    void Function() onFocus,
    void Function(List<String> targets) onOpenTargets,
  ) async {
    // Bind to an ephemeral port (port 0)
    final serverSocket = await ServerSocket.bind(
      InternetAddress.loopbackIPv4,
      0,
    );

    // Write the assigned port to the lock file
    await _lockFile!.writeAsString(serverSocket.port.toString(), flush: true);

    // Listen for incoming connections
    serverSocket.listen((socket) {
      // A request can arrive split across chunks, so collect it all and only
      // act once the sender has closed its side.
      final request = StringBuffer();
      socket.listen(
        (data) => request.write(utf8.decode(data, allowMalformed: true)),
        onError: (_) {},
        onDone: () => _handleRequest(request.toString(), onFocus, onOpenTargets),
        cancelOnError: true,
      );
    });
  }

  static void _handleRequest(
    String request,
    void Function() onFocus,
    void Function(List<String> targets) onOpenTargets,
  ) {
    final message = request.trim();
    if (message.isEmpty) return;

    if (message == _focusCommand) {
      onFocus();
      return;
    }

    final List<String> targets = _decodeTargets(message);
    if (targets.isEmpty) {
      log('Ignoring unrecognised single instance request: $message',
          level: LogLevel.warning);
      return;
    }

    // Opening a file should also bring the window forward, which is what the
    // user asked for by double clicking it.
    onFocus();
    onOpenTargets(targets);
  }

  /// Recovers the file list from a request, or an empty list if it is not one.
  static List<String> _decodeTargets(String message) {
    try {
      final decoded = jsonDecode(message);
      if (decoded is! Map) return const [];
      if (decoded['cmd'] != 'open') return const [];
      final paths = decoded['paths'];
      if (paths is! List) return const [];
      return [for (final path in paths) if (path is String) path];
    } catch (_) {
      return const [];
    }
  }

  static Future<void> dispose() async {
    if (_lockFile != null) {
      try {
        if (await _lockFile!.exists()) {
          await _lockFile!.delete();
        }
      } catch (e) {
        // Ignore errors during disposal
      }
    }
  }
}