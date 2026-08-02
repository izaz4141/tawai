import 'dart:async';
import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/logger.dart';

class DuplicateFinderPage extends StatefulWidget {
  const DuplicateFinderPage({super.key});

  @override
  State<DuplicateFinderPage> createState() => _DuplicateFinderPageState();
}

class _DuplicateFinderPageState extends State<DuplicateFinderPage> {
  bool _checkFingerprint = true;
  bool _checkMbid = true;
  bool _checkFileSizeDuration = true;
  bool _checkTitleArtist = false;
  bool _scanning = false;
  List<DuplicateGroup> _groups = [];
  bool _hasScanned = false;
  String? _error;

  Future<void> _scan() async {
    setState(() {
      _scanning = true;
      _error = null;
      _groups = [];
    });
    try {
      final response = await BridgeService.instance.findDuplicates(
        checkFingerprint: _checkFingerprint,
        checkMbid: _checkMbid,
        checkFileSizeDuration: _checkFileSizeDuration,
        checkTitleArtist: _checkTitleArtist,
      );
      if (!mounted) return;
      if (response != null) {
        setState(() {
          _groups = response.groups;
          _hasScanned = true;
          _scanning = false;
        });
      } else {
        setState(() {
          _error = 'Failed to scan for duplicates';
          _scanning = false;
        });
      }
    } catch (e) {
      log('duplicate_finder: scan error: $e', isError: true);
      if (mounted) {
        setState(() {
          _error = e.toString();
          _scanning = false;
        });
      }
    }
  }

  IconData _methodIcon(String method) {
    switch (method) {
      case 'file_size_duration':
        return Icons.storage;
      case 'mbid':
        return Icons.music_note;
      case 'fingerprint':
        return Icons.fingerprint;
      case 'title_artist':
        return Icons.title;
      default:
        return Icons.help_outline;
    }
  }

  String _methodLabel(String method) {
    switch (method) {
      case 'file_size_duration':
        return 'File Size + Duration';
      case 'mbid':
        return 'MusicBrainz ID';
      case 'fingerprint':
        return 'Audio Fingerprint';
      case 'title_artist':
        return 'Title + Artist';
      default:
        return method;
    }
  }

  Color _confidenceColor(double confidence) {
    if (confidence >= 0.95) return Colors.green;
    if (confidence >= 0.8) return Colors.orange;
    return Colors.red;
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final isDesktop = AppTheme.isDesktop(context);

    return Scaffold(
      appBar: AppBar(
        title: Text('Duplicate Finder',
            style: isDesktop ? textTheme.titleLarge : null),
      ),
      body: Column(
        children: [
          _buildOptionsPanel(colors, textTheme, isDesktop),
          const Divider(height: 1),
          if (_scanning)
            const Expanded(
              child: Center(child: CircularProgressIndicator()),
            )
          else if (_error != null)
            Expanded(
              child: Center(
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: Text(_error!,
                      style: textTheme.bodyMedium
                          ?.copyWith(color: colors.error)),
                ),
              ),
            )
          else if (!_hasScanned)
            Expanded(
              child: Center(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(Icons.copy_all,
                        size: 64, color: colors.onSurfaceVariant),
                    const SizedBox(height: 16),
                    Text('Configure options above and tap Scan',
                        style: textTheme.bodyLarge
                            ?.copyWith(color: colors.onSurfaceVariant)),
                  ],
                ),
              ),
            )
          else if (_groups.isEmpty)
            Expanded(
              child: Center(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(Icons.check_circle,
                        size: 64, color: colors.primary),
                    const SizedBox(height: 16),
                    Text('No duplicates found!',
                        style: textTheme.titleMedium),
                  ],
                ),
              ),
            )
          else
            Expanded(
              child: ListView.builder(
                padding: const EdgeInsets.all(8),
                itemCount: _groups.length,
                itemBuilder: (context, index) {
                  final group = _groups[index];
                  return _buildGroupCard(group, colors, textTheme);
                },
              ),
            ),
        ],
      ),
    );
  }

  Widget _buildOptionsPanel(
      ColorScheme colors, TextTheme textTheme, bool isDesktop) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 8, 16, 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              Expanded(
                child: Wrap(
                  spacing: 16,
                  runSpacing: 4,
                  children: [
                    FilterChip(
                      label: const Text('File Size + Duration'),
                      selected: _checkFileSizeDuration,
                      onSelected: (v) => setState(() => _checkFileSizeDuration = v),
                    ),
                    FilterChip(
                      label: const Text('Audio Fingerprint'),
                      selected: _checkFingerprint,
                      onSelected: (v) => setState(() => _checkFingerprint = v),
                    ),
                    FilterChip(
                      label: const Text('MusicBrainz ID'),
                      selected: _checkMbid,
                      onSelected: (v) => setState(() => _checkMbid = v),
                    ),
                    FilterChip(
                      label: const Text('Title + Artist'),
                      selected: _checkTitleArtist,
                      onSelected: (v) => setState(() => _checkTitleArtist = v),
                    ),
                  ],
                ),
              ),
              FilledButton.icon(
                onPressed: _scanning ? null : _scan,
                icon: _scanning
                    ? const SizedBox(
                        width: 16,
                        height: 16,
                        child: CircularProgressIndicator(strokeWidth: 2))
                    : const Icon(Icons.search, size: 18),
                label: Text(_scanning ? 'Scanning...' : 'Scan'),
              ),
            ],
          ),
          if (_hasScanned)
            Padding(
              padding: const EdgeInsets.only(top: 4),
              child: Text(
                '${_groups.fold<int>(0, (s, g) => s + g.tracks.length)} duplicates in ${_groups.length} groups',
                style: textTheme.bodySmall
                    ?.copyWith(color: colors.onSurfaceVariant),
              ),
            ),
        ],
      ),
    );
  }

  Widget _buildGroupCard(
      DuplicateGroup group, ColorScheme colors, TextTheme textTheme) {
    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: ExpansionTile(
        leading: Icon(_methodIcon(group.method), color: colors.primary),
        title: Text(
          '${_methodLabel(group.method)} — ${group.tracks.length} tracks',
          style: textTheme.bodyMedium,
        ),
        subtitle: Row(
          children: [
            Icon(Icons.circle,
                size: 10,
                color: _confidenceColor(group.confidence)),
            const SizedBox(width: 4),
            Text(
              '${(group.confidence * 100).toInt()}% confidence',
              style: textTheme.bodySmall,
            ),
          ],
        ),
        children: group.tracks.map((track) {
          return ListTile(
            dense: true,
            leading: Icon(Icons.audiotrack,
                size: 20, color: colors.onSurfaceVariant),
            title: Text(track.title,
                maxLines: 1, overflow: TextOverflow.ellipsis),
            subtitle: Text(
              '${track.artist} • ${track.album}',
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: textTheme.bodySmall,
            ),
            trailing: IconButton(
              icon: Icon(Icons.delete_outline, size: 18, color: colors.error),
              tooltip: 'Delete track',
              onPressed: () {
                // TODO: implement delete
              },
            ),
          );
        }).toList(),
      ),
    );
  }
}
