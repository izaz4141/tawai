class FilterableList<T> {
  List<T> all = [];

  final bool Function(T item, String query) matchesSearch;
  final bool Function(T item, String? source)? matchesSource;
  final int Function(T a, T b)? compare;

  FilterableList({
    required this.matchesSearch,
    this.matchesSource,
    this.compare,
  });

  List<T> filtered(String query, String? source) {
    var result = all;
    if (query.isNotEmpty) {
      final q = query.toLowerCase();
      result = result.where((e) => matchesSearch(e, q)).toList();
    }
    if (source != null && matchesSource != null) {
      result = result.where((e) => matchesSource!(e, source)).toList();
    }
    if (compare != null) {
      result = result.toList()..sort(compare!);
    }
    return result;
  }
}
