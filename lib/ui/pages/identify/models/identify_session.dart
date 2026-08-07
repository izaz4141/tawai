import 'package:tawai/src/bindings/bindings.dart';

class IdentifySession {
  final TrackInfo track;
  final RecordingInfo recording;
  String? selectedReleaseMbid;
  List<ReleaseTrackInfo> releaseTracks;
  ReleaseTrackInfo? correctTrack;
  bool loadingReleaseTracks;
  bool applied;

  IdentifySession({
    required this.track,
    required this.recording,
    this.selectedReleaseMbid,
    this.releaseTracks = const [],
    this.correctTrack,
    this.loadingReleaseTracks = false,
    this.applied = false,
  });
}
