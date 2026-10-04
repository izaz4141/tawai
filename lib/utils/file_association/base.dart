/// Offers this build as a handler for audio files.
///
/// Linux declares the types in its desktop entry and macOS in its Info.plist,
/// both of which ship with the package, so nothing is left for the running
/// process to do there. Only Windows needs registry entries written at runtime,
/// so [register] is a no-op on the other platforms.
///
/// macOS is out of scope here: its Info.plist is unchanged and this does not
/// claim otherwise.
///
/// Use `FileAssociationServiceFactory.create()` to obtain one.
abstract class FileAssociationService {
  /// Whether this build is currently registered to handle the formats.
  bool isRegistered();

  /// Adds this build to the list of programs the OS offers for the formats.
  Future<void> register();
}