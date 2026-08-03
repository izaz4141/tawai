class RecommendationSource {
  final String key;
  final String displayName;
  final String recType;

  const RecommendationSource({
    required this.key,
    required this.displayName,
    required this.recType,
  });

  static const List<RecommendationSource> all = [
    RecommendationSource(
      key: 'recommendation:listenbrainz_weekly-explo',
      displayName: 'Weekly Exploration',
      recType: 'weekly-exploration',
    ),
    RecommendationSource(
      key: 'recommendation:listenbrainz_year',
      displayName: 'Year in Music',
      recType: 'year',
    ),
    RecommendationSource(
      key: 'recommendation:listenbrainz_top',
      displayName: 'Top Recommendations',
      recType: 'top',
    ),
    RecommendationSource(
      key: 'recommendation:listenbrainz_raw',
      displayName: 'Raw Recommendations',
      recType: 'raw',
    ),
    RecommendationSource(
      key: 'recommendation:listenbrainz_similar',
      displayName: 'Similar Artists',
      recType: 'similar',
    ),
    RecommendationSource(
      key: 'recommendation:listenbrainz_weekly',
      displayName: 'Weekly',
      recType: 'weekly',
    ),
    RecommendationSource(
      key: 'recommendation:listenbrainz_daily',
      displayName: 'Daily',
      recType: 'daily',
    ),
  ];

  static final Map<String, RecommendationSource> byKey = {
    for (final s in all) s.key: s,
  };

  static RecommendationSource? fromKey(String key) => byKey[key];

  static String? getDisplayName(String key) => byKey[key]?.displayName;

  static String? getRecType(String key) => byKey[key]?.recType;

  static bool isRecommendationSource(String key) =>
      key.startsWith('recommendation:');

  /// Whether a track's source type supports downloading (recommendations and
  /// live previews from discovery).
  static bool isDownloadable(String sourceType) =>
      isRecommendationSource(sourceType) || sourceType == 'preview';

  static String? findKeyByDisplayName(String displayName) {
    for (final s in all) {
      if (s.displayName == displayName) return s.key;
    }
    return null;
  }

  static bool isDisplayName(String name) =>
      all.any((s) => s.displayName == name);
}
