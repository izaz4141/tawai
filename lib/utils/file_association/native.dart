import 'dart:io';

import 'package:win32_registry/win32_registry.dart';

import 'package:tawai/models/audio_format.dart';
import 'package:tawai/utils/logger.dart';
import 'package:tawai/utils/file_association/base.dart';

/// Offers the running build to Windows as a handler for audio files.
///
/// The formats go into the *Open with* list only, so a handler the user already
/// had is left alone. Nothing here writes a default value under
/// `Software\Classes\.mp3`, and `UserChoice` is hash protected anyway, so the
/// user always picks Tawai themselves.
class _FileAssociationNative implements FileAssociationService {
  static const String _description = 'Tawai Audio File';
  static const String _progId = 'Tawai.Audio';

  // Spelled out rather than built by interpolation: a raw string cannot
  // interpolate, and a non-raw one needs every backslash escaped.
  static const String _progIdPath = r'Software\Classes\Tawai.Audio';
  static const String _commandPath =
      r'Software\Classes\Tawai.Audio\shell\open\command';

  /// `HKCU\Software\Classes\.<ext>\OpenWithProgids` for every extension.
  static List<String> get _openWithPaths => [
    for (final ext in audioExtensions) 'Software\\Classes\\$ext\\OpenWithProgids',
  ];

  @override
  bool isRegistered() {
    // Linux and macOS register through their desktop entry and Info.plist
    // instead, so there is nothing this process can check or change there.
    // Reporting registered keeps the startup check from calling [register],
    // which would only no-op.
    if (Platform.isLinux || Platform.isMacOS) return true;

    // Everything below is FFI, so on another platform it is a hard failure
    // rather than a catchable exception.
    if (!Platform.isWindows) return false;

    final command = _command();
    try {
      if (_readDefault(_commandPath) != command) return false;
      return _openWithPaths.every((path) => _readValue(path, _progId) == '');
    } catch (e) {
      log('File association lookup failed: $e', level: LogLevel.error);
      return false;
    }
  }

  @override
  Future<void> register() async {
    if (Platform.isLinux || Platform.isMacOS) return;
    if (!Platform.isWindows) return;
    try {
      final command = _command();
      final exe = Platform.resolvedExecutable;
      _registerProgId(exe, command);
      _registerOpenWith();
    } catch (e, s) {
      log('Failed to register file association: $e\n$s',
          level: LogLevel.error);
      rethrow;
    }

    if (!isRegistered()) {
      log('File association did not take effect', level: LogLevel.error);
    }
  }

  /// The command the shell runs to hand us a file.
  String _command() => '"${Platform.resolvedExecutable}" "%1"';

  String? _readDefault(String path) => _readValue(path, '');

  String? _readValue(String path, String name) {
    final key = CURRENT_USER.open(
      path,
      config: const RegistryOpenConfig(access: RegistryAccess.read),
    );
    try {
      return (key.getValue(name) as StringValue?)?.value;
    } finally {
      key.close();
    }
  }

  // `create` is not implied by calling `create()`: supplying a config drops
  // that method's own default, and a config without it opens rather than
  // creates, which fails on the first run when the key does not exist yet.
  RegistryKey _create(String path) => CURRENT_USER.create(
    path,
    config: const RegistryOpenConfig(
      access: RegistryAccess.readWrite,
      create: true,
    ),
  );

  void _write(String path, Map<String, RegistryValue> values) {
    final key = _create(path);
    try {
      for (final entry in values.entries) {
        key.setValue(entry.key, entry.value);
      }
    } finally {
      key.close();
    }
  }

  /// Registers through a ProgId of our own, so the extension keys only gain an
  /// `OpenWithProgids` entry and keep whatever default they had.
  void _registerProgId(String exe, String command) {
    _write(_progIdPath, {
      // The empty name is the key's default value.
      '': const StringValue(_description),
      'FriendlyTypeName': const StringValue(_description),
      // Lets Explorer describe the file types using this ProgId.
      'Extensions': MultiStringValue([...audioExtensions]),
      'Icon': StringValue('$exe,0'),
    });

    _write(_commandPath, {'': StringValue(command)});
  }

  void _registerOpenWith() {
    for (final path in _openWithPaths) {
      // An empty value is how Windows records "this ProgId can open this type".
      _write(path, {_progId: const StringValue('')});
    }
  }
}

FileAssociationService getFileAssociationService() => _FileAssociationNative();