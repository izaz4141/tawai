import 'package:tawai/src/bindings/bindings.dart';

extension TrackInfoJson on TrackInfo {
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
