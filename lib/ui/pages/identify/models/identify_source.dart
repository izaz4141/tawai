import 'package:tawai/src/bindings/bindings.dart';

sealed class IdentifySource {
  const IdentifySource();
}

class UnidentifiedSource extends IdentifySource {
  const UnidentifiedSource();
}

class LibrarySource extends IdentifySource {
  final LibrarySourceInfo info;
  const LibrarySource(this.info);

  @override
  bool operator ==(Object other) =>
      other is LibrarySource && info.id == other.info.id;

  @override
  int get hashCode => info.id.hashCode;
}
