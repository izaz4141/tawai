class FsEntry {
  final String name;
  final String path;
  final bool isDir;
  final int? size;

  const FsEntry({
    required this.name,
    required this.path,
    required this.isDir,
    this.size,
  });

  factory FsEntry.fromJson(Map<String, dynamic> json) => FsEntry(
    name: json['name'] as String? ?? '',
    path: json['path'] as String? ?? '',
    isDir: json['is_dir'] as bool? ?? false,
    size: (json['size'] as num?)?.toInt(),
  );
}

class FsListing {
  final String path;
  final String? parent;
  final List<FsEntry> entries;

  const FsListing({required this.path, this.parent, required this.entries});

  factory FsListing.fromJson(Map<String, dynamic> json) => FsListing(
    path: json['path'] as String? ?? '',
    parent: json['parent'] as String?,
    entries: (json['entries'] as List<dynamic>? ?? [])
        .map((e) => FsEntry.fromJson(e as Map<String, dynamic>))
        .toList(),
  );
}
