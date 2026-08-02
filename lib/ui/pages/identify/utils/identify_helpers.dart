import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/pages/identify/models/identify_result.dart';
import 'package:tawai/ui/pages/identify/models/identify_session.dart';

String normalize(String s) =>
    s.toLowerCase().replaceAll(RegExp(r'[^\w\s]'), '').trim();

String formatDuration(double? secs) {
  if (secs == null) return '';
  final total = secs.round();
  final m = total ~/ 60;
  final s = total % 60;
  return '${m.toString().padLeft(2, '0')}:${s.toString().padLeft(2, '0')}';
}

ReleaseTrackInfo? autoAssignCorrectTrack({
  required String trackTitle,
  required double? trackDuration,
  required int? trackPosition,
  int? trackDiscNumber,
  required List<ReleaseTrackInfo> releaseTracks,
}) {
  if (releaseTracks.isEmpty) return null;
  final originalTitle = normalize(trackTitle);
  final originalDuration = trackDuration;

  ReleaseTrackInfo? best;
  int bestScore = -1;
  for (final track in releaseTracks) {
    int score = 0;
    final trackTitle = normalize(track.title);
    if (trackTitle == originalTitle) {
      score += 100;
    } else if (trackTitle.contains(originalTitle) ||
        originalTitle.contains(trackTitle)) {
      score += 50;
    }
    if (originalDuration != null && track.durationSecs != null) {
      final diff = (originalDuration - track.durationSecs!).abs();
      if (diff < 3) {
        score += 30;
      } else if (diff < 10) {
        score += 15;
      }
    }
    if (trackPosition != null && track.position == trackPosition) score += 10;
    if (trackDiscNumber != null &&
        track.discNumber != null &&
        track.discNumber == trackDiscNumber) {
      score += 10;
    }
    if (score > bestScore) {
      bestScore = score;
      best = track;
    }
  }
  return best ?? releaseTracks.first;
}

bool tagsDiffer(
  IdentifiedUserTrack ut,
  ReleaseTrackInfo? remoteTrack,
  IdentifyAlbumResult album,
) {
  if (remoteTrack == null) return false;
  if (normalize(ut.title) != normalize(remoteTrack.title)) return true;
  if (album.releaseArtist != null &&
      normalize(ut.artistsString) != normalize(album.releaseArtist!)) {
    return true;
  }
  if (album.albumTitle.isNotEmpty &&
      normalize(ut.albumTitle) != normalize(album.albumTitle)) {
    return true;
  }
  return false;
}

void groupSessionIntoAlbum(
  IdentifySession session,
  TrackInfo track,
  Map<String, IdentifyAlbumResult> albumResults,
) {
  final albumKey = IdentifyAlbumResult.albumKey(track.albumId);
  final existing = albumResults[albumKey];
  final userTrack = IdentifiedUserTrack(
    trackId: track.id,
    title: track.title,
    artistsString: track.artistsString,
    albumTitle: track.albumTitle,
    trackNum: track.trackNum,
    discNum: track.discNum,
    artists: track.artists,
    lyrics: track.lyrics,
    mbidRecording: track.mbidRecording,
    isSession: true,
    sessionId: track.id,
  );

  if (existing != null) {
    existing.userTracks.removeWhere((t) => t.trackId == track.id);
    existing.userTracks.add(userTrack);
    existing.expanded = true;
    if (existing.albumMbid == null && track.albumMbid != null) {
      existing.albumMbid = track.albumMbid;
    }
  } else {
    albumResults[albumKey] = IdentifyAlbumResult(
      albumTitle: track.albumTitle,
      sourceAlbumId: track.albumId,
      userTracks: [userTrack],
      expanded: true,
      albumMbid: track.albumMbid,
    );
  }
}

void groupIdentifiedTracks(
  List<TrackInfo> identifiedTracks,
  Map<String, IdentifyAlbumResult> albumResults,
) {
  final byAlbum = <String, List<TrackInfo>>{};
  for (final t in identifiedTracks) {
    byAlbum.putIfAbsent(t.albumId, () => []).add(t);
  }
  for (final entry in byAlbum.entries) {
    final albumTracks = entry.value;
    final first = albumTracks.first;
    final userTracks = albumTracks
        .map(
          (t) => IdentifiedUserTrack(
            trackId: t.id,
            title: t.title,
            artistsString: t.artistsString,
            albumTitle: t.albumTitle,
            trackNum: t.trackNum,
            discNum: t.discNum,
            artists: t.artists,
            lyrics: t.lyrics,
            mbidRecording: t.mbidRecording,
            isSession: false,
          ),
        )
        .toList();
    final albumKey = IdentifyAlbumResult.albumKey(first.albumId);
    final existing = albumResults[albumKey];
    if (existing != null) {
      for (final ut in userTracks) {
        existing.userTracks.removeWhere((t) => t.trackId == ut.trackId);
      }
      existing.userTracks.addAll(userTracks);
    } else {
      albumResults[albumKey] = IdentifyAlbumResult(
        albumTitle: first.albumTitle,
        sourceAlbumId: first.albumId,
        userTracks: userTracks,
        expanded: false,
        albumMbid: first.albumMbid,
      );
    }
  }
}
