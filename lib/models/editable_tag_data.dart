/// The complete editable tag field set collected by the tag editor, carried as
/// one value through the bridge layers (rinf signals and REST payloads).
///
/// Every field is always sent: an empty string, null, empty list, or `0` means
/// "remove the stored value" (the write handlers use full-set semantics).
///
/// The read-only fields are intentionally absent and are never written: file
/// path, length, encoder, replaygain track gain/peak, and the derived original
/// year.
class EditableTagData {
  final String title;
  final String artist;
  final List<String> artists;
  final String album;
  final String albumArtist;
  final List<String> albumArtists;
  final List<String> genres;
  final int trackNumber;
  final int totalTracks;
  final int discNumber;
  final int totalDiscs;
  final String? releaseDate;
  final String? originalDate;
  final String media;
  final String script;
  final String recordLabel;
  final String releaseCountry;
  final List<String> releaseType;
  final String isrc;
  final String mbidRecording;
  final String mbidTrack;
  final String mbidArtist;
  final String mbidRelease;
  final String mbidReleaseArtist;
  final String mbidReleaseGroup;
  final String artistSort;
  final String albumArtistSort;
  final String? comment;
  final String? copyright;
  final String? lyrics;
  final List<int>? cover;
  final bool clearCover;

  const EditableTagData({
    required this.title,
    required this.artist,
    required this.artists,
    required this.album,
    required this.albumArtist,
    required this.albumArtists,
    required this.genres,
    required this.trackNumber,
    required this.totalTracks,
    required this.discNumber,
    required this.totalDiscs,
    this.releaseDate,
    this.originalDate,
    required this.media,
    required this.script,
    required this.recordLabel,
    required this.releaseCountry,
    required this.releaseType,
    required this.isrc,
    required this.mbidRecording,
    required this.mbidTrack,
    required this.mbidArtist,
    required this.mbidRelease,
    required this.mbidReleaseArtist,
    required this.mbidReleaseGroup,
    required this.artistSort,
    required this.albumArtistSort,
    this.comment,
    this.copyright,
    this.lyrics,
    this.cover,
    this.clearCover = false,
  });
}
