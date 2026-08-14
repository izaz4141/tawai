class LibraryFilters {
  String query;
  final Set<String> sources;
  final Set<String> genres;
  final Set<String> years;

  LibraryFilters({
    this.query = '',
    Set<String>? sources,
    Set<String>? genres,
    Set<String>? years,
  }) : sources = sources ?? {},
       genres = genres ?? {},
       years = years ?? {};

  bool get hasActiveFilters =>
      sources.isNotEmpty || genres.isNotEmpty || years.isNotEmpty;

  int get activeCount => sources.length + genres.length + years.length;

  LibraryFilters copy() => LibraryFilters(
    query: query,
    sources: {...sources},
    genres: {...genres},
    years: {...years},
  );
}

class LibraryFilterOptions {
  final List<String> sources;
  final List<String> genres;
  final List<String> years;

  const LibraryFilterOptions({
    this.sources = const [],
    this.genres = const [],
    this.years = const [],
  });
}

class FilterableList<T> {
  List<T> all = [];

  final bool Function(T item, String query) matchesSearch;
  final bool Function(T item, LibraryFilters filters)? matchesFilters;
  final int Function(T a, T b)? compare;

  FilterableList({
    required this.matchesSearch,
    this.matchesFilters,
    this.compare,
  });

  List<T> filtered(LibraryFilters filters) {
    var result = all;
    if (filters.query.isNotEmpty) {
      final q = filters.query.toLowerCase();
      result = result.where((e) => matchesSearch(e, q)).toList();
    }
    if (matchesFilters != null) {
      result = result.where((e) => matchesFilters!(e, filters)).toList();
    }
    if (compare != null) {
      result = result.toList()..sort(compare!);
    }
    return result;
  }
}
