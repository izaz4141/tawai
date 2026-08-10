import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/pages/search/search_types.dart';
import 'package:tawai/utils/helper.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/ui/pages/search/widgets/search_result_tile.dart';
import 'package:tawai/ui/widgets/mini_player.dart';
import 'package:tawai/ui/pages/search/modals/release_picker_dialog.dart';
import 'package:tawai/ui/pages/search/modals/track_picker_dialog.dart';

enum _EnhancedStep { choosingMbResult, viewingResults }

class EnhancedSearchView extends StatefulWidget {
  final String downloadSource;
  final List<RecordingInfo> recordings;
  final bool searching;
  final Set<String> loadingUrls;
  final Future<List<SearchResultItem>> Function(String query) onSearch;
  final void Function(SearchResultItem entry) onDownload;

  const EnhancedSearchView({
    super.key,
    required this.downloadSource,
    required this.recordings,
    this.searching = false,
    this.loadingUrls = const {},
    required this.onSearch,
    required this.onDownload,
  });

  @override
  State<EnhancedSearchView> createState() => _EnhancedSearchViewState();
}

class _EnhancedSearchViewState extends State<EnhancedSearchView> {
  List<SearchResultItem> _results = [];
  bool _searchingSource = false;
  _EnhancedStep _step = _EnhancedStep.choosingMbResult;

  @override
  void didUpdateWidget(EnhancedSearchView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.recordings != oldWidget.recordings) {
      setState(() {
        _results = [];
        _searchingSource = false;
        _step = _EnhancedStep.choosingMbResult;
      });
    }
  }

  void _selectRecording(RecordingInfo rec) {
    if (rec.releases.isEmpty) {
      _searchFromData(rec.artist, rec.title);
      return;
    }
    showReleasePicker(
      context: context,
      recording: rec,
      onSelected: (release) => _onReleaseSelected(rec, release),
    );
  }

  Future<void> _onReleaseSelected(
    RecordingInfo rec,
    ReleaseInfo release,
  ) async {
    setState(() => _searchingSource = true);
    final data = await BridgeService.instance.getReleaseTracks(release.id);
    if (!mounted) return;
    setState(() => _searchingSource = false);
    if (data.tracks.isEmpty) {
      _searchFromData(rec.artist, rec.title);
      return;
    }
    showTrackPicker(
      context: context,
      release: release,
      tracks: data.tracks,
      onSelected: (track) => _onTrackSelected(rec, track),
    );
  }

  void _onTrackSelected(RecordingInfo rec, ReleaseTrackInfo track) {
    _searchFromData(rec.artist, track.title);
  }

  Future<void> _searchFromData(String artist, String title) async {
    setState(() {
      _searchingSource = true;
      _results = [];
    });
    final query = '$artist $title';
    final results = await widget.onSearch(query);
    if (mounted) {
      setState(() {
        _results = results;
        _searchingSource = false;
        _step = _EnhancedStep.viewingResults;
      });
    }
  }

  Widget _buildCoverArt(RecordingInfo rec) {
    final colors = Theme.of(context).colorScheme;
    if (rec.cover != null) {
      return ClipRRect(
        borderRadius: BorderRadius.circular(AppTheme.radiusSM),
        child: Image.network(
          rec.cover!,
          width: AppTheme.iconXL,
          height: AppTheme.iconXL,
          fit: BoxFit.cover,
          errorBuilder: (_, _, _) => Container(
            width: AppTheme.iconXL,
            height: AppTheme.iconXL,
            color: colors.surfaceContainerHighest,
            child: Icon(
              Icons.music_note,
              size: AppTheme.iconMD,
              color: colors.onSurfaceVariant,
            ),
          ),
        ),
      );
    }
    return Container(
      width: AppTheme.iconXL,
      height: AppTheme.iconXL,
      color: colors.surfaceContainerHighest,
      child: Icon(
        Icons.music_note,
        size: AppTheme.iconMD,
        color: colors.onSurfaceVariant,
      ),
    );
  }

  Widget _buildRecordingCard(RecordingInfo rec) {
    final colors = Theme.of(context).colorScheme;
    return Card(
      child: InkWell(
        borderRadius: BorderRadius.circular(AppTheme.radiusMD),
        onTap: () => _selectRecording(rec),
        child: Padding(
          padding: const EdgeInsets.all(AppTheme.spaceSM),
          child: Row(
            children: [
              _buildCoverArt(rec),
              const SizedBox(width: AppTheme.spaceMD),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      rec.title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.titleSmall?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    SizedBox(height: AppTheme.spaceXS / 2),
                    Text(
                      rec.artist,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                    if (rec.releases.isNotEmpty)
                      Text(
                        '${rec.releases.length} release${rec.releases.length > 1 ? 's' : ''}',
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: colors.onSurfaceVariant,
                        ),
                      ),
                    if (rec.durationSecs != null)
                      Text(
                        formatDuration(rec.durationSecs),
                        style: Theme.of(
                          context,
                        ).textTheme.bodySmall?.copyWith(color: colors.outline),
                      ),
                  ],
                ),
              ),
              const SizedBox(width: AppTheme.spaceSM),
              Icon(
                Icons.arrow_forward_ios,
                size: AppTheme.iconSM,
                color: colors.outline,
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildResultsList(List<SearchResultItem> entries) {
    if (entries.isEmpty) {
      return const Center(child: Text('No results.'));
    }
    return ListView.builder(
      itemCount: entries.length + 1,
      itemBuilder: (context, index) {
        if (index == entries.length) return const MiniPlayerSpacer();
        final entry = entries[index];
        return SearchResultTile(
          entry: entry,
          loadingUrls: widget.loadingUrls,
          onDownload: () => widget.onDownload(entry),
        );
      },
    );
  }

  @override
  Widget build(BuildContext context) {
    final sourceLabel = widget.downloadSource == 'slskd'
        ? 'Soulseek'
        : 'YouTube';
    final textTheme = Theme.of(context).textTheme;

    if (_step == _EnhancedStep.viewingResults) {
      return Column(
        children: [
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: AppTheme.spaceMD),
            child: Row(
              children: [
                IconButton(
                  icon: const Icon(Icons.arrow_back),
                  tooltip: 'Back to MusicBrainz results',
                  onPressed: () => setState(() {
                    _step = _EnhancedStep.choosingMbResult;
                    _results = [];
                  }),
                ),
                Expanded(
                  child: Text(
                    _results.isEmpty
                        ? 'Searching...'
                        : '$sourceLabel results for selected track',
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                ),
              ],
            ),
          ),
          Expanded(
            child: _searchingSource
                ? const Center(child: CircularProgressIndicator())
                : _buildResultsList(_results),
          ),
        ],
      );
    }

    return Column(
      children: [
        Expanded(
          child: widget.searching
              ? const Center(child: CircularProgressIndicator())
              : widget.recordings.isEmpty
              ? Center(
                  child: Text(
                    'Search MusicBrainz to discover music.',
                    style: textTheme.bodyMedium,
                  ),
                )
              : ListView.builder(
                  padding: const EdgeInsets.symmetric(
                    horizontal: AppTheme.spaceMD,
                  ),
                  itemCount: widget.recordings.length + 1,
                  itemBuilder: (_, i) {
                    if (i == widget.recordings.length) {
                      return const MiniPlayerSpacer();
                    }
                    return Padding(
                      padding: const EdgeInsets.only(bottom: AppTheme.spaceXS),
                      child: _buildRecordingCard(widget.recordings[i]),
                    );
                  },
                ),
        ),
      ],
    );
  }
}
