import 'package:tawai/src/bindings/bindings.dart';

extension TrackInfoJson on TrackInfo {
  static TrackInfo fromJson(Map<String, dynamic> json) => TrackInfo(
    id: json['id'] as String,
    title: json['title'] as String,
    albumId: json['album_id'] as String? ?? '',
    albumTitle: json['album_title'] as String? ?? '',
    artistsString: json['artists_string'] as String? ?? '',
    artists: [
      for (final a in json['artists'] as List<dynamic>? ?? [])
        ArtistInfoJson.fromJson(a as Map<String, dynamic>),
    ],
    trackNum: json['track_num'] as int?,
    discNum: json['disc_num'] as int?,
    durationSecs: (json['duration_secs'] as num?)?.toDouble(),
    filePath: json['file_path'] as String? ?? '',
    fileSize: json['file_size'] as int?,
    bitrate: json['bitrate'] as int?,
    mbidRecording: json['mbid_recording'] as String?,
    artistMbid: json['artist_mbid'] as String?,
    albumMbid: json['album_mbid'] as String?,
    lyrics: json['lyrics'] as String?,
    releaseDate: json['release_date'] as String?,
    trackGain: (json['track_gain'] as num?)?.toDouble(),
    trackPeak: (json['track_peak'] as num?)?.toDouble(),
    source: json['source'] as String? ?? 'remote',
    sourceType: json['source_type'] as String? ?? '',
    genres: [
      for (final g in json['genres'] as List<dynamic>? ?? []) g as String,
    ],
  );

  Map<String, dynamic> toJson() => {
    'id': id,
    'title': title,
    'album_id': albumId,
    'album_title': albumTitle,
    'artists_string': artistsString,
    'artists': [for (final a in artists) a.toJson()],
    'track_num': trackNum,
    'disc_num': discNum,
    'duration_secs': durationSecs,
    'file_path': filePath,
    'file_size': fileSize,
    'bitrate': bitrate,
    'mbid_recording': mbidRecording,
    'artist_mbid': artistMbid,
    'album_mbid': albumMbid,
    'lyrics': lyrics,
    'release_date': releaseDate,
    'track_gain': trackGain,
    'track_peak': trackPeak,
    'source': source,
    'source_type': sourceType,
    'genres': genres,
  };
}

extension ArtistInfoJson on ArtistInfo {
  static ArtistInfo fromJson(Map<String, dynamic> json) => ArtistInfo(
    id: json['id'] as String,
    name: json['name'] as String,
    sortName: json['sort_name'] as String?,
    mbid: json['mbid'] as String?,
    thumbnailUrl: json['thumbnail_url'] as String?,
    albumCount: (json['album_count'] as num?)?.toInt() ?? 0,
    trackCount: (json['track_count'] as num?)?.toInt() ?? 0,
  );

  Map<String, dynamic> toJson() => {
    'id': id,
    'name': name,
    'sort_name': sortName,
    'mbid': mbid,
    'thumbnail_url': thumbnailUrl,
    'album_count': albumCount,
    'track_count': trackCount,
  };
}
