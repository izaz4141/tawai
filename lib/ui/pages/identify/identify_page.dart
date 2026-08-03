import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/widgets/app_shell.dart';
import 'package:tawai/ui/pages/identify/controller/identify_controller.dart';
import 'package:tawai/ui/pages/identify/widgets/left_panel.dart';
import 'package:tawai/ui/pages/identify/widgets/release_picker_dialog.dart';
import 'package:tawai/ui/pages/identify/widgets/right_panel.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/app_snackbar.dart';
import 'package:tawai/ui/widgets/dialog/identify_dialog.dart';

class IdentifyPage extends StatefulWidget {
  const IdentifyPage({super.key});

  @override
  State<IdentifyPage> createState() => _IdentifyPageState();
}

class _IdentifyPageState extends State<IdentifyPage> {
  late final IdentifyController _controller;
  bool loadingMetadata = false;

  @override
  void initState() {
    super.initState();
    _controller = IdentifyController();
    _controller.loadSources();
    _controller.loadTracks();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  Future<void> _onFingerprint() async {
    final track = _controller.selectedTrack;
    if (track == null) return;
    setState(() => loadingMetadata = true);
    try {
      final rec = await _controller.fingerprintTrack(track);
      if (!mounted) return;
      if (rec == null) {
        AppSnackBar.show(
          context,
          'Fingerprint lookup returned no results',
          type: SnackType.error,
        );
        return;
      }
      _showReleasePickerForSession(track, rec);
    } finally {
      if (mounted) setState(() => loadingMetadata = false);
    }
  }

  Future<void> _onLookup() async {
    final track = _controller.selectedTrack;
    if (track == null) return;
    setState(() => loadingMetadata = true);
    try {
      final results = await _controller.lookupTrack(track);
      if (!mounted) return;
      if (results.isEmpty) {
        AppSnackBar.show(
          context,
          'Lookup returned no results',
          type: SnackType.error,
        );
      } else if (results.length == 1) {
        _showReleasePickerForSession(track, results.first);
      } else {
        showIdentifyDialog(
          context,
          currentTitle: track.title,
          currentArtist: track.artistsString,
          currentAlbum: track.albumTitle,
          onReleaseSelected: (recording, release) {
            _controller.addSessionResult(track, recording);
            final session = _controller.sessions[track.id];
            if (session != null) {
              _controller.onReleaseSelected(session, release);
            }
          },
        );
      }
    } finally {
      if (mounted) setState(() => loadingMetadata = false);
    }
  }

  void _showReleasePickerForSession(TrackInfo track, RecordingInfo rec) {
    if (rec.releases.length <= 1) {
      final session = _controller.sessions[track.id];
      if (session != null && rec.releases.isNotEmpty) {
        _controller.onReleaseSelected(session, rec.releases.first);
      }
      return;
    }
    showReleasePickerDialog(
      context: context,
      recording: rec,
      onSelected: (release) {
        final session = _controller.sessions[track.id];
        if (session != null) {
          _controller.onReleaseSelected(session, release);
        }
      },
    );
  }

  @override
  Widget build(BuildContext context) {
    final isDesktop = AppTheme.isDesktop(context);

    return AnimatedBuilder(
      animation: _controller,
      builder: (context, _) {
        final rightPanel = _controller.albumResults.isNotEmpty
            ? RightPanel(controller: _controller)
            : Center(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      Icons.album,
                      size: 64,
                      color: Theme.of(context).colorScheme.outline,
                    ),
                    const SizedBox(height: 16),
                    Text(
                      'No identified tracks',
                      style: Theme.of(context).textTheme.bodyLarge,
                    ),
                  ],
                ),
              );

        return AppShell(
          title: 'Identify Tracks',
          body: isDesktop
              ? Row(
                  children: [
                    SizedBox(
                      width: 360,
                      child: LeftPanel(
                        controller: _controller,
                        loadingMetadata: loadingMetadata,
                        onFingerprint: loadingMetadata ? null : _onFingerprint,
                        onLookup: loadingMetadata ? null : _onLookup,
                      ),
                    ),
                    Container(
                      width: 1,
                      color: Theme.of(context).colorScheme.outlineVariant,
                    ),
                    Expanded(child: rightPanel),
                  ],
                )
              : Column(
                  children: [
                    SizedBox(
                      height: 240,
                      child: LeftPanel(
                        controller: _controller,
                        loadingMetadata: loadingMetadata,
                        onFingerprint: loadingMetadata ? null : _onFingerprint,
                        onLookup: loadingMetadata ? null : _onLookup,
                      ),
                    ),
                    Container(
                      height: 1,
                      color: Theme.of(context).colorScheme.outlineVariant,
                    ),
                    Expanded(child: rightPanel),
                  ],
                ),
        );
      },
    );
  }
}
