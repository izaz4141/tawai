import 'package:flutter/material.dart';
import 'package:tawai/ui/pages/identify/controller/identify_controller.dart';
import 'package:tawai/ui/pages/identify/widgets/source_selector.dart';
import 'package:tawai/ui/pages/identify/widgets/track_list_view.dart';
import 'package:tawai/ui/pages/identify/widgets/action_buttons.dart';

class LeftPanel extends StatelessWidget {
  final IdentifyController controller;
  final bool fingerprinting;
  final bool lookingUp;
  final VoidCallback? onFingerprint;
  final VoidCallback? onLookup;

  const LeftPanel({
    super.key,
    required this.controller,
    this.fingerprinting = false,
    this.lookingUp = false,
    this.onFingerprint,
    this.onLookup,
  });

  bool get _showActions => controller.selectedTrack != null;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        SourceSelector(
          selected: controller.selectedSource,
          librarySources: controller.librarySources,
          onChanged: controller.onSourceChanged,
        ),
        const Divider(height: 1),
        Expanded(
          child: TrackListView(
            tracks: controller.tracks,
            loading: controller.loadingTracks,
            selectedTrackId: controller.selectedTrack?.id,
            onSelect: controller.onSelectTrack,
            onRefresh: controller.loadTracks,
          ),
        ),
        if (_showActions) ...[
          const Divider(height: 1),
          ActionButtons(
            fingerprinting: fingerprinting,
            lookingUp: lookingUp,
            onFingerprint: onFingerprint,
            onLookup: onLookup,
          ),
        ],
      ],
    );
  }
}
