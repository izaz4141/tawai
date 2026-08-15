import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/widgets/app_shell.dart';
import 'package:tawai/ui/widgets/mini_player.dart';
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
  bool fingerprinting = false;
  bool lookingUp = false;

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
    setState(() => fingerprinting = true);
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
      if (mounted) setState(() => fingerprinting = false);
    }
  }

  Future<void> _onLookup() async {
    final track = _controller.selectedTrack;
    if (track == null) return;
    setState(() => lookingUp = true);
    try {
      final results = await _controller.lookupTrack(track);
      if (!mounted) return;
      if (results.length == 1) {
        _showReleasePickerForSession(track, results.first);
        return;
      }
      showIdentifyDialog(
        context,
        currentTitle: track.title,
        currentArtist: track.artistsString,
        currentAlbum: track.albumTitle,
        initialRecordings: results,
        onReleaseSelected: (recording, release) {
          _controller.addSessionResult(track, recording);
          final session = _controller.sessions[track.id];
          if (session != null) {
            _controller.onReleaseSelected(session, release);
          }
        },
      );
    } finally {
      if (mounted) setState(() => lookingUp = false);
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
    final anyLoading = fingerprinting || lookingUp;

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
                      size: AppTheme.iconXXL * AppTheme.iconScale(context),
                      color: Theme.of(context).colorScheme.outline,
                    ),
                    SizedBox(
                      height:
                          AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
                    ),
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
                    Column(
                      children: [
                        Expanded(
                          child: SizedBox(
                            width:
                                AppTheme.spaceXL *
                                15 *
                                AppTheme.widthScale(context),
                            child: LeftPanel(
                              controller: _controller,
                              fingerprinting: fingerprinting,
                              lookingUp: lookingUp,
                              onFingerprint: anyLoading ? null : _onFingerprint,
                              onLookup: anyLoading ? null : _onLookup,
                            ),
                          ),
                        ),
                        const MiniPlayerSpacer(),
                      ],
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
                      height:
                          AppTheme.spaceXL * 10 * AppTheme.spaceScale(context),
                      child: LeftPanel(
                        controller: _controller,
                        fingerprinting: fingerprinting,
                        lookingUp: lookingUp,
                        onFingerprint: anyLoading ? null : _onFingerprint,
                        onLookup: anyLoading ? null : _onLookup,
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
