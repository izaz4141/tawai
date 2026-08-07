import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/pages/identify/controller/identify_controller.dart';
import 'package:tawai/ui/pages/identify/models/identify_result.dart';

import 'package:tawai/ui/pages/identify/modals/track_comparison_sheet.dart';
import 'package:tawai/ui/pages/identify/utils/identify_helpers.dart';
import 'package:tawai/ui/pages/identify/widgets/album_track_row.dart';
import 'package:tawai/ui/pages/identify/widgets/release_picker_dialog.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/dialog/search_lyrics.dart';
import 'package:url_launcher/url_launcher.dart';

class AlbumResultCard extends StatelessWidget {
  final String albumKey;
  final IdentifyAlbumResult album;
  final IdentifyController controller;

  const AlbumResultCard({
    super.key,
    required this.albumKey,
    required this.album,
    required this.controller,
  });

  Set<int> get _releasePositions =>
      album.releaseTracks.map((t) => t.position ?? 0).toSet();

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final scale = AppTheme.spaceScale(context);

    return Container(
      decoration: BoxDecoration(
        border: Border.all(color: colors.outlineVariant.withAlpha(80)),
        borderRadius: BorderRadius.circular(AppTheme.radiusMD),
      ),
      margin: EdgeInsets.all(AppTheme.spaceSM * scale),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _buildHeader(context),
          if (album.expanded) ...[
            const Divider(height: 1),
            if (album.loadingMbid)
              const Padding(
                padding: EdgeInsets.all(16),
                child: Center(child: CircularProgressIndicator(strokeWidth: 2)),
              )
            else if (album.releaseTracks.isNotEmpty)
              _buildReleaseTrackList(context)
            else
              Column(
                children: [
                  for (final ut in album.userTracks)
                    _buildTrackRow(
                      context,
                      position: ut.trackNum ?? 0,
                      ownedTracks: [ut],
                    ),
                ],
              ),
            if (!album.loadingMbid) const SizedBox(height: AppTheme.spaceSM),
          ],
        ],
      ),
    );
  }

  Widget _buildHeader(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final scale = AppTheme.spaceScale(context);

    return InkWell(
      onTap: () => controller.onToggleAlbumExpand(albumKey),
      child: Padding(
        padding: EdgeInsets.all(AppTheme.spaceMD * scale),
        child: Row(
          children: [
            Icon(
              album.expanded ? Icons.expand_less : Icons.expand_more,
              size: AppTheme.iconMD,
              color: colors.onSurfaceVariant,
            ),
            const SizedBox(width: AppTheme.spaceSM),
            Icon(
              Icons.album,
              size: AppTheme.iconMD,
              color: album.allIdentified
                  ? Colors.green
                  : (album.hasSession ? Colors.orange : colors.error),
            ),
            const SizedBox(width: AppTheme.spaceSM),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    album.albumTitle,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.titleSmall?.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  Text(
                    '${album.userTracks.length} track${album.userTracks.length == 1 ? '' : 's'} · '
                    '${album.unsavedCount} unsaved',
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: colors.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildReleaseTrackList(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final positions = album.releaseTracks.map((t) => t.position ?? 0).toList();
    final maxPos = positions.isEmpty
        ? 0
        : positions.reduce((a, b) => a > b ? a : b);
    final entries = <Widget>[];

    if (album.editingTrackId != null) {
      final editingTrack = album.userTracks
          .where((t) => t.trackId == album.editingTrackId)
          .firstOrNull;
      entries.add(
        Container(
          color: colors.primaryContainer.withAlpha(40),
          padding: EdgeInsets.symmetric(
            horizontal: AppTheme.spaceMD * AppTheme.spaceScale(context),
            vertical: AppTheme.spaceSM,
          ),
          child: Row(
            children: [
              Expanded(
                child: Text(
                  'Select correct track for: ${editingTrack?.title ?? ''}',
                  style: Theme.of(
                    context,
                  ).textTheme.labelMedium?.copyWith(color: colors.primary),
                ),
              ),
              TextButton(
                onPressed: () => controller.onStopEditingTrack(albumKey),
                child: const Text('Done'),
              ),
            ],
          ),
        ),
      );
    }

    for (int i = 1; i <= maxPos; i++) {
      final remoteTrack = positions.contains(i)
          ? album.releaseTracks.firstWhere((t) => t.position == i)
          : null;
      if (remoteTrack == null) continue;
      final owned = album.userTracks
          .where((t) => t.assignedPosition == i)
          .toList();
      entries.add(
        _buildTrackRow(
          context,
          position: i,
          remoteTrack: remoteTrack,
          ownedTracks: owned,
        ),
      );
    }

    final unmatched = album.userTracks
        .where((t) => !_releasePositions.contains(t.assignedPosition))
        .toList();
    if (unmatched.isNotEmpty) {
      for (final ut in unmatched) {
        entries.add(
          _buildTrackRow(
            context,
            position: ut.trackNum ?? 0,
            ownedTracks: [ut],
          ),
        );
      }
    }

    return Column(children: entries);
  }

  Widget _buildTrackRow(
    BuildContext context, {
    required int position,
    ReleaseTrackInfo? remoteTrack,
    List<IdentifiedUserTrack> ownedTracks = const [],
  }) {
    final colors = Theme.of(context).colorScheme;
    final isEditing = album.editingTrackId != null;
    final editingTrack = isEditing
        ? album.userTracks
              .where((t) => t.trackId == album.editingTrackId)
              .firstOrNull
        : null;
    final hasRelease = album.releaseTracks.isNotEmpty;
    final isThisSelected =
        ownedTracks.isNotEmpty &&
        controller.selectedTrack?.id == ownedTracks.first.trackId;

    IconData icon;
    Color iconColor;

    if (isEditing) {
      final isSelected = editingTrack?.assignedPosition == position;
      icon = isSelected ? Icons.radio_button_checked : Icons.radio_button_off;
      iconColor = isSelected ? colors.primary : colors.outline;
    } else if (remoteTrack == null && hasRelease) {
      icon = Icons.warning_amber_rounded;
      iconColor = Colors.orange;
    } else if (ownedTracks.isEmpty) {
      icon = Icons.circle_outlined;
      iconColor = colors.outline.withAlpha(80);
    } else {
      final ut = ownedTracks.first;
      if (ut.isSession && !ut.applied) {
        icon = Icons.fiber_manual_record;
        iconColor = colors.error;
      } else if (tagsDiffer(ut, remoteTrack, album)) {
        icon = Icons.sync_problem;
        iconColor = Colors.yellow.shade700;
      } else {
        icon = Icons.check_circle;
        iconColor = Colors.green;
      }
    }

    String title;
    String? subtitle;
    Color? subtitleColor;
    Color? titleColor;
    FontWeight? titleWeight;
    var showWarningIcon = false;

    if (remoteTrack != null) {
      title = remoteTrack.title;

      if (ownedTracks.isEmpty) {
        titleWeight = FontWeight.normal;
        titleColor = colors.outline;
        subtitle = remoteTrack.durationSecs != null
            ? formatDuration(remoteTrack.durationSecs)
            : null;
      } else {
        titleWeight = FontWeight.w600;

        if (!isEditing) {
          final ut = ownedTracks.first;
          final titleDiffers = tagsDiffer(ut, remoteTrack, album);

          if (ut.isSession && !ut.applied) {
            subtitle = 'Unsaved';
          } else if (titleDiffers) {
            subtitle = remoteTrack.durationSecs != null
                ? 'Was: ${ut.title} · ${formatDuration(remoteTrack.durationSecs)}'
                : 'Was: ${ut.title}';
            subtitleColor = Colors.orange;
            showWarningIcon = true;
          } else if (remoteTrack.durationSecs != null) {
            subtitle = formatDuration(remoteTrack.durationSecs);
          }
        }
      }
    } else {
      final ut = ownedTracks.first;
      title = ut.title;
      titleWeight = FontWeight.w600;

      if (hasRelease) {
        subtitle = 'No matching MB track';
        subtitleColor = Colors.orange;
      } else if (ut.isSession && !ut.applied) {
        subtitle = 'Unsaved';
      }
    }

    Widget? trailing;
    if (remoteTrack != null && ownedTracks.isNotEmpty && !isEditing) {
      final ut = ownedTracks.first;
      trailing = Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          IconButton(
            icon: Icon(
              Icons.info_outline,
              size: AppTheme.iconMD * AppTheme.iconScale(context),
              color: colors.primary,
            ),
            onPressed: () => _showDetails(context, ut, remoteTrack),
            style: IconButton.styleFrom(
              padding: EdgeInsets.zero,
              tapTargetSize: MaterialTapTargetSize.shrinkWrap,
            ),
          ),
          PopupMenuButton<String>(
            icon: Icon(
              Icons.more_vert,
              size: AppTheme.iconMD * AppTheme.iconScale(context),
              color: colors.onSurfaceVariant,
            ),
            style: IconButton.styleFrom(
              padding: EdgeInsets.zero,
              tapTargetSize: MaterialTapTargetSize.shrinkWrap,
            ),
            onSelected: (value) =>
                _onMenuSelected(context, value, ut, remoteTrack, albumKey),
            itemBuilder: (_) => [
              PopupMenuItem(
                value: 'reselect_release',
                enabled: ut.isSession || ut.mbidRecording != null,
                child: const Text('Reselect Release'),
              ),
              const PopupMenuItem(
                value: 'search_lyrics',
                child: Text('Search Lyrics'),
              ),
              const PopupMenuItem(
                value: 'open_browser',
                child: Text('Open in Browser'),
              ),
            ],
          ),
        ],
      );
    }

    return AlbumTrackRow(
      position: position,
      title: title,
      subtitle: subtitle,
      subtitleColor: subtitleColor,
      icon: icon,
      iconColor: iconColor,
      titleWeight: titleWeight,
      titleColor: titleColor,
      warningIcon: showWarningIcon,
      isSelected: isThisSelected,
      trailing: trailing,
      onTap: isEditing && editingTrack != null
          ? () => controller.onAssignPosition(
              albumKey,
              editingTrack.trackId,
              position,
            )
          : !isEditing && ownedTracks.isNotEmpty
          ? () {
              final ut = ownedTracks.first;
              controller.onSelectTrack(
                controller.buildTrackInfoForSelection(
                  ut,
                  album,
                  remoteTrack: remoteTrack,
                ),
              );
            }
          : null,
      onLongPress: !isEditing && ownedTracks.isNotEmpty
          ? () => controller.onStartEditingTrack(
              albumKey,
              ownedTracks.first.trackId,
            )
          : null,
    );
  }

  Future<void> _onMenuSelected(
    BuildContext context,
    String value,
    IdentifiedUserTrack ut,
    ReleaseTrackInfo remoteTrack,
    String albumKey,
  ) async {
    switch (value) {
      case 'reselect_release':
        await _onReselectRelease(context, ut, albumKey);
        break;
      case 'search_lyrics':
        _onSearchLyrics(context, ut, albumKey);
        break;
      case 'open_browser':
        final mbid = ut.isSession ? remoteTrack.id : ut.mbidRecording;
        if (mbid != null) {
          launchUrl(Uri.parse('https://musicbrainz.org/recording/$mbid'));
        }
        break;
    }
  }

  Future<void> _onReselectRelease(
    BuildContext context,
    IdentifiedUserTrack ut,
    String albumKey,
  ) async {
    final recording = await controller.fetchRecordingForTrack(ut);
    if (!context.mounted || recording == null) return;

    if (recording.releases.length <= 1) {
      if (recording.releases.isNotEmpty) {
        final session = ut.sessionId != null
            ? controller.sessions[ut.sessionId]
            : null;
        if (session != null) {
          await controller.onReleaseSelected(session, recording.releases.first);
        } else {
          await controller.updateAlbumRelease(
            trackId: ut.trackId,
            releaseId: recording.releases.first.id,
            sourceAlbumId: album.sourceAlbumId,
            albumTitle: ut.albumTitle,
          );
        }
      }
      return;
    }

    if (!context.mounted) return;
    showReleasePickerDialog(
      context: context,
      recording: recording,
      onSelected: (release) async {
        final session = ut.sessionId != null
            ? controller.sessions[ut.sessionId]
            : null;
        if (session != null) {
          await controller.onReleaseSelected(session, release);
        } else {
          await controller.updateAlbumRelease(
            trackId: ut.trackId,
            releaseId: release.id,
            sourceAlbumId: album.sourceAlbumId,
            albumTitle: ut.albumTitle,
          );
        }
      },
    );
  }

  void _onSearchLyrics(
    BuildContext context,
    IdentifiedUserTrack ut,
    String albumKey,
  ) {
    final query = '${ut.title} ${ut.artistsString}'.trim();
    showDialog(
      context: context,
      builder: (_) => SearchLyricsDialog(
        initialQuery: query,
        onSelect: (lyrics) {
          controller.setTrackLyrics(albumKey, ut.trackId, lyrics);
        },
      ),
    );
  }

  Future<void> _showDetails(
    BuildContext context,
    IdentifiedUserTrack userTrack,
    ReleaseTrackInfo remoteTrack,
  ) async {
    remoteTrack = album.releaseTracks.firstWhere(
      (t) => t.id == remoteTrack.id,
      orElse: () => remoteTrack,
    );
    final isSession = userTrack.isSession && userTrack.sessionId != null;

    TrackInfo currentTrack;
    String recordingArtist;

    if (isSession) {
      final session = controller.sessions[userTrack.sessionId];
      if (session == null) return;
      currentTrack = session.track;
      recordingArtist = session.recording.artist;
    } else {
      recordingArtist = album.releaseArtist ?? '';
      currentTrack = controller.buildTrackInfoForSelection(
        userTrack,
        album,
        remoteTrack: remoteTrack,
      );
    }

    if (!context.mounted) return;

    final result = await showTrackComparisonSheet(
      context: context,
      controller: controller,
      currentTrack: currentTrack,
      correctTrack: remoteTrack,
      album: album,
      recordingArtist: recordingArtist,
    );
    if (!context.mounted) return;
    if (result?.applied == true) {
      controller.updateUserTrackAfterApply(
        albumKey: albumKey,
        trackId: userTrack.trackId,
        title: result!.title ?? userTrack.title,
        artist: result.artist ?? userTrack.artistsString,
        albumTitle: result.album ?? userTrack.albumTitle,
        trackNum: result.trackNum ?? userTrack.trackNum,
        discNum: result.discNum ?? userTrack.discNum,
        mbidRecording: result.mbidRecording ?? userTrack.mbidRecording,
        mbidAlbum: result.mbidAlbum,
        mbidArtist: result.mbidArtist,
        lyrics: result.lyrics ?? userTrack.lyrics,
        releaseDate: result.releaseDate,
      );
    }
  }
}
