import 'dart:async';
import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/app_snackbar.dart';
import 'package:tawai/ui/widgets/dialog/language_picker_dialog.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/logger.dart';
import 'package:tawai/utils/settings.dart';

class LyricsManagerPage extends StatefulWidget {
  const LyricsManagerPage({super.key});

  @override
  State<LyricsManagerPage> createState() => _LyricsManagerPageState();
}

class _LyricsSearchDialog extends StatefulWidget {
  final String initialQuery;
  const _LyricsSearchDialog({required this.initialQuery});

  @override
  State<_LyricsSearchDialog> createState() => _LyricsSearchDialogState();
}

class _LyricsSearchDialogState extends State<_LyricsSearchDialog> {
  final _queryController = TextEditingController();
  List<LyricsResult> _results = [];
  bool _loading = false;

  @override
  void initState() {
    super.initState();
    _queryController.text = widget.initialQuery;
    _doSearch();
  }

  @override
  void dispose() {
    _queryController.dispose();
    super.dispose();
  }

  Future<void> _doSearch() async {
    final query = _queryController.text.trim();
    if (query.isEmpty) return;
    setState(() => _loading = true);
    try {
      final results = await BridgeService.instance.searchLyrics(query: query);
      if (mounted) setState(() => _results = results);
    } catch (e) {
      log('search error: $e', isError: true);
    }
    if (mounted) setState(() => _loading = false);
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    return AlertDialog(
      title: const Text('Search LRCLIB'),
      content: SizedBox(
        width: AppTheme.dialogWidth(context),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _queryController,
                    decoration: const InputDecoration(
                      hintText: 'Search query...',
                      isDense: true,
                      contentPadding: EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 10,
                      ),
                    ),
                    onSubmitted: (_) => _doSearch(),
                  ),
                ),
                SizedBox(
                  width: AppTheme.spaceSM * AppTheme.spaceScale(context),
                ),
                IconButton(
                  onPressed: _loading ? null : _doSearch,
                  icon: _loading
                      ? SizedBox(
                          width:
                              AppTheme.spaceLG * AppTheme.spaceScale(context),
                          height:
                              AppTheme.spaceLG * AppTheme.spaceScale(context),
                          child: const CircularProgressIndicator(
                            strokeWidth: 2,
                          ),
                        )
                      : const Icon(Icons.search),
                ),
              ],
            ),
            SizedBox(height: AppTheme.spaceMD * AppTheme.spaceScale(context)),
            if (_results.isEmpty && !_loading)
              Padding(
                padding: const EdgeInsets.symmetric(vertical: 24),
                child: Text(
                  'No results found',
                  style: textTheme.bodyMedium?.copyWith(
                    color: colors.onSurfaceVariant,
                  ),
                ),
              )
            else
              Flexible(
                child: ListView.builder(
                  shrinkWrap: true,
                  itemCount: _results.length,
                  itemBuilder: (context, index) {
                    final r = _results[index];
                    final preview = r.lyrics.length > 80
                        ? '${r.lyrics.substring(0, 80)}...'
                        : r.lyrics;
                    return ListTile(
                      dense: true,
                      title: Text(
                        '${r.title} — ${r.artist}',
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      subtitle: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            r.album,
                            style: textTheme.bodySmall?.copyWith(
                              color: colors.onSurfaceVariant,
                            ),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                          SizedBox(
                            height:
                                AppTheme.spaceXS * AppTheme.spaceScale(context),
                          ),
                          Text(
                            preview,
                            style: textTheme.bodySmall?.copyWith(
                              fontStyle: FontStyle.italic,
                              color: colors.onSurfaceVariant,
                            ),
                          ),
                        ],
                      ),
                      trailing: Chip(
                        avatar: Icon(
                          r.synced ? Icons.timer : Icons.text_fields,
                          size: AppTheme.iconSM * AppTheme.iconScale(context),
                        ),
                        label: Text(
                          r.synced ? 'Synced' : 'Plain',
                          style: TextStyle(
                            fontSize:
                                AppTheme.textSM * AppTheme.textScale(context),
                          ),
                        ),
                        visualDensity: VisualDensity.compact,
                      ),
                      onTap: () => Navigator.of(context).pop(r),
                    );
                  },
                ),
              ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
      ],
    );
  }
}

class _LyricsManagerPageState extends State<LyricsManagerPage> {
  List<LibrarySourceInfo> _sources = [];
  List<TrackInfo> _allTracks = [];
  List<TrackInfo> _filteredTracks = [];
  TrackInfo? _selectedTrack;
  bool _loadingTracks = false;
  String? _selectedSourceId;
  final _searchController = TextEditingController();
  final _lyricsController = TextEditingController();
  bool _saving = false;
  Timer? _debounce;

  @override
  void initState() {
    super.initState();
    _loadSources();
    _searchController.addListener(_onSearchChanged);
  }

  @override
  void dispose() {
    _searchController.removeListener(_onSearchChanged);
    _searchController.dispose();
    _lyricsController.dispose();
    _debounce?.cancel();
    super.dispose();
  }

  void _onSearchChanged() {
    _debounce?.cancel();
    _debounce = Timer(const Duration(milliseconds: 300), _filterTracks);
  }

  void _filterTracks() {
    final query = _searchController.text.toLowerCase().trim();
    setState(() {
      if (query.isEmpty) {
        _filteredTracks = List.from(_allTracks);
      } else {
        _filteredTracks = _allTracks.where((t) {
          return t.title.toLowerCase().contains(query) ||
              t.artistsString.toLowerCase().contains(query);
        }).toList();
      }
    });
  }

  Future<void> _loadSources() async {
    final userId = SettingsManager.currentUserId.value ?? '';
    try {
      final sources = await BridgeService.instance.listEditableSources(userId);
      if (!mounted) return;
      setState(() {
        _sources = sources;
      });
      if (sources.isNotEmpty && _selectedSourceId == null) {
        _selectedSourceId = sources.first.id;
        _loadTracks(sources.first.id);
      }
    } catch (e) {
      log('lyrics_manager: loadSources error: $e', isError: true);
    }
  }

  Future<void> _loadTracks(String sourceId) async {
    setState(() {
      _loadingTracks = true;
      _selectedSourceId = sourceId;
      _selectedTrack = null;
      _lyricsController.clear();
    });
    try {
      final tracks = await BridgeService.instance.listTracksBySource(sourceId);
      if (!mounted) return;
      setState(() {
        _allTracks = tracks;
        _filteredTracks = List.from(tracks);
        _loadingTracks = false;
      });
    } catch (e) {
      log('lyrics_manager: loadTracks error: $e', isError: true);
      if (mounted) {
        setState(() {
          _allTracks = [];
          _filteredTracks = [];
          _loadingTracks = false;
        });
      }
    }
  }

  void _selectTrack(TrackInfo track) {
    setState(() {
      _selectedTrack = track;
      _lyricsController.text = track.lyrics ?? '';
    });
  }

  Future<void> _showSearchDialog() async {
    if (_selectedTrack == null) return;
    final result = await showDialog<LyricsResult>(
      context: context,
      builder: (ctx) => _LyricsSearchDialog(
        initialQuery:
            '${_selectedTrack!.title} ${_selectedTrack!.artistsString}',
      ),
    );
    if (result != null && mounted) {
      setState(() {
        _lyricsController.text = result.lyrics;
      });
    }
  }

  Future<void> _romajizeLyrics() async {
    if (_selectedTrack == null) return;
    final text = _lyricsController.text.trim();
    if (text.isEmpty) return;

    final lang = await LanguagePickerDialog.show(
      context,
      title: 'Romajize Language',
    );
    if (lang == null || !mounted) return;

    try {
      final response = await BridgeService.instance.romajizeLyrics(
        lyrics: text,
        synced: false,
        lang: lang,
      );
      if (!mounted) return;
      if (response?.error == null && response?.romajized != null) {
        setState(() => _lyricsController.text = response!.romajized);
        if (context.mounted) {
          AppSnackBar.show(
            context,
            'Lyrics romajized',
            type: SnackType.success,
          );
        }
      }
    } catch (e) {
      log('romajize error: $e', isError: true);
    }
  }

  Future<void> _saveLyrics() async {
    if (_selectedTrack == null) return;
    setState(() => _saving = true);
    try {
      final response = await BridgeService.instance.writeTrackLyrics(
        trackId: _selectedTrack!.id,
        lyrics: _lyricsController.text,
        synced: false,
      );
      if (!mounted) return;
      if (response?.success == true) {
        if (context.mounted) {
          AppSnackBar.show(context, 'Lyrics saved', type: SnackType.success);
        }
      }
    } catch (e) {
      log('lyrics_manager: saveLyrics error: $e', isError: true);
    }
    if (mounted) {
      setState(() => _saving = false);
    }
  }

  Future<void> _clearLyrics() async {
    if (_selectedTrack == null) return;
    setState(() {
      _lyricsController.text = '';
      _saving = true;
    });
    try {
      final response = await BridgeService.instance.writeTrackLyrics(
        trackId: _selectedTrack!.id,
        lyrics: '',
        synced: false,
      );
      if (!mounted) return;
      if (response?.success == true) {
        if (context.mounted) {
          AppSnackBar.show(context, 'Lyrics cleared', type: SnackType.success);
        }
      }
    } catch (e) {
      log('lyrics_manager: clearLyrics error: $e', isError: true);
    }
    if (mounted) {
      setState(() => _saving = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final isDesktop = AppTheme.isDesktop(context);

    return Scaffold(
      appBar: AppBar(
        title: Text(
          'Lyrics Manager',
          style: isDesktop ? textTheme.titleLarge : null,
        ),
      ),
      body: isDesktop
          ? Row(
              children: [
                SizedBox(
                  width: AppTheme.spaceXL * 15 * AppTheme.widthScale(context),
                  child: _buildTrackPanel(colors, textTheme),
                ),
                const VerticalDivider(width: 1),
                Expanded(child: _buildEditorPanel(colors, textTheme)),
              ],
            )
          : Column(
              children: [
                SizedBox(
                  height: AppTheme.spaceXXL * 9 * AppTheme.spaceScale(context),
                  child: _buildTrackPanel(colors, textTheme),
                ),
                const Divider(height: 1),
                Expanded(child: _buildEditorPanel(colors, textTheme)),
              ],
            ),
    );
  }

  Widget _buildTrackPanel(ColorScheme colors, TextTheme textTheme) {
    return Column(
      children: [
        Padding(
          padding: EdgeInsets.fromLTRB(
            AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
            AppTheme.spaceSM * AppTheme.spaceScale(context),
            AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
            AppTheme.spaceXS * AppTheme.spaceScale(context),
          ),
          child: DropdownButtonFormField<String>(
            value: _selectedSourceId,
            decoration: InputDecoration(
              labelText: 'Source',
              isDense: true,
              contentPadding: EdgeInsets.symmetric(
                horizontal: AppTheme.spaceMD * AppTheme.spaceScale(context),
                vertical: AppTheme.spaceMD * AppTheme.spaceScale(context),
              ),
            ),
            items: _sources.map((s) {
              return DropdownMenuItem(value: s.id, child: Text(s.name));
            }).toList(),
            onChanged: (v) {
              if (v != null) _loadTracks(v);
            },
          ),
        ),
        Padding(
          padding: EdgeInsets.fromLTRB(
            AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
            AppTheme.spaceXS * AppTheme.spaceScale(context),
            AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
            AppTheme.spaceXS * AppTheme.spaceScale(context),
          ),
          child: TextField(
            controller: _searchController,
            decoration: InputDecoration(
              hintText: 'Search tracks...',
              prefixIcon: const Icon(Icons.search),
              isDense: true,
              contentPadding: EdgeInsets.symmetric(
                horizontal: AppTheme.spaceMD * AppTheme.spaceScale(context),
                vertical: AppTheme.spaceMD * AppTheme.spaceScale(context),
              ),
            ),
          ),
        ),
        Expanded(
          child: _loadingTracks
              ? const Center(child: CircularProgressIndicator())
              : _filteredTracks.isEmpty
              ? Center(
                  child: Text(
                    'No tracks',
                    style: textTheme.bodyMedium?.copyWith(
                      color: colors.onSurfaceVariant,
                    ),
                  ),
                )
              : ListView.builder(
                  itemCount: _filteredTracks.length,
                  itemBuilder: (context, index) {
                    final track = _filteredTracks[index];
                    final hasLyrics =
                        track.lyrics != null && track.lyrics!.isNotEmpty;
                    final isSelected = _selectedTrack?.id == track.id;
                    return ListTile(
                      dense: true,
                      selected: isSelected,
                      leading: Icon(
                        hasLyrics
                            ? Icons.music_note
                            : Icons.music_note_outlined,
                        size: AppTheme.iconMD * AppTheme.iconScale(context),
                        color: hasLyrics
                            ? colors.primary
                            : colors.onSurfaceVariant,
                      ),
                      title: Text(
                        track.title,
                        style: textTheme.bodyMedium,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      subtitle: Text(
                        track.artistsString,
                        style: textTheme.bodySmall,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      onTap: () => _selectTrack(track),
                    );
                  },
                ),
        ),
      ],
    );
  }

  Widget _buildEditorPanel(ColorScheme colors, TextTheme textTheme) {
    if (_selectedTrack == null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.edit_note,
              size: AppTheme.iconXXL * AppTheme.iconScale(context),
              color: colors.onSurfaceVariant,
            ),
            SizedBox(
              height: AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
            ),
            Text(
              'Select a track to edit lyrics',
              style: textTheme.bodyLarge?.copyWith(
                color: colors.onSurfaceVariant,
              ),
            ),
          ],
        ),
      );
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: EdgeInsets.fromLTRB(
            AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
            AppTheme.spaceMD * AppTheme.spaceScale(context),
            AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
            AppTheme.spaceXS * AppTheme.spaceScale(context),
          ),
          child: Text(_selectedTrack!.title, style: textTheme.titleMedium),
        ),
        Padding(
          padding: EdgeInsets.fromLTRB(
            AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
            0,
            AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
            AppTheme.spaceSM * AppTheme.spaceScale(context),
          ),
          child: Text(
            _selectedTrack!.artistsString,
            style: textTheme.bodyMedium?.copyWith(
              color: colors.onSurfaceVariant,
            ),
          ),
        ),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16),
          child: Row(
            children: [
              FilledButton.icon(
                onPressed: _showSearchDialog,
                icon: Icon(
                  Icons.search,
                  size: AppTheme.iconSM * AppTheme.iconScale(context),
                ),
                label: const Text('Search LRCLIB'),
              ),
            ],
          ),
        ),
        SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
        Expanded(
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: TextField(
              controller: _lyricsController,
              maxLines: null,
              expands: true,
              textAlignVertical: TextAlignVertical.top,
              style: TextStyle(
                fontFamily: 'monospace',
                fontSize: AppTheme.textSM * AppTheme.textScale(context),
              ),
              decoration: InputDecoration(
                hintText: 'Enter lyrics here...',
                border: const OutlineInputBorder(),
                contentPadding: EdgeInsets.all(
                  AppTheme.spaceMD * AppTheme.spaceScale(context),
                ),
              ),
            ),
          ),
        ),
        Padding(
          padding: EdgeInsets.fromLTRB(
            AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
            AppTheme.spaceSM * AppTheme.spaceScale(context),
            AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
            AppTheme.spaceMD * AppTheme.spaceScale(context),
          ),
          child: Row(
            children: [
              FilledButton.icon(
                onPressed: _saving ? null : _saveLyrics,
                icon: _saving
                    ? SizedBox(
                        width:
                            AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
                        height:
                            AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
                        child: const CircularProgressIndicator(strokeWidth: 2),
                      )
                    : Icon(
                        Icons.save,
                        size: AppTheme.iconSM * AppTheme.iconScale(context),
                      ),
                label: const Text('Save'),
              ),
              SizedBox(width: AppTheme.spaceSM * AppTheme.spaceScale(context)),
              OutlinedButton.icon(
                onPressed: _saving ? null : _clearLyrics,
                icon: Icon(
                  Icons.clear,
                  size: AppTheme.iconSM * AppTheme.iconScale(context),
                ),
                label: const Text('Clear'),
              ),
              SizedBox(width: AppTheme.spaceSM * AppTheme.spaceScale(context)),
              OutlinedButton.icon(
                onPressed: _saving ? null : _romajizeLyrics,
                icon: Icon(
                  Icons.translate,
                  size: AppTheme.iconSM * AppTheme.iconScale(context),
                ),
                label: const Text('Romajize'),
              ),
            ],
          ),
        ),
      ],
    );
  }
}
