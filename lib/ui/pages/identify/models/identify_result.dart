import 'package:tawai/src/bindings/bindings.dart';

class IdentifiedUserTrack {
  final String trackId;
  final String title;
  final String artistsString;
  final String albumTitle;
  final int? trackNum;
  final int? discNum;
  final List<ArtistInfo> artists;
  final String? lyrics;
  final String? mbidRecording;
  bool isSession;
  final String? sessionId;
  bool applied;

  int? assignedPosition;

  IdentifiedUserTrack({
    required this.trackId,
    required this.title,
    this.artistsString = '',
    this.albumTitle = '',
    this.trackNum,
    this.discNum,
    this.artists = const [],
    this.lyrics,
    this.mbidRecording,
    this.isSession = false,
    this.sessionId,
    this.applied = false,
    int? assignedPosition,
  }) : assignedPosition = assignedPosition ?? trackNum;
}

class IdentifyAlbumResult {
  String albumTitle;
  final String? sourceAlbumId;
  String? albumMbid;
  List<ReleaseTrackInfo> releaseTracks;
  List<IdentifiedUserTrack> userTracks;
  bool expanded;
  bool loadingMbid;
  String? editingTrackId;
  String? releaseDate;
  String? releaseArtist;
  String? releaseArtistMbid;
  String? albumDisambiguation;

  IdentifyAlbumResult({
    required this.albumTitle,
    this.sourceAlbumId,
    this.albumMbid,
    this.releaseTracks = const [],
    this.userTracks = const [],
    this.expanded = false,
    this.loadingMbid = false,
    this.editingTrackId,
    this.releaseDate,
    this.releaseArtist,
    this.releaseArtistMbid,
    this.albumDisambiguation,
  });

  static String albumKey(String? albumId) => 'album_${albumId ?? ''}';

  int get unsavedCount => userTracks.where((t) => t.isSession && !t.applied).length;
  bool get hasSession => userTracks.any((t) => t.isSession);
  bool get allIdentified => userTracks.every((t) => !t.isSession);
}
