import 'package:flutter/material.dart';
import 'package:tawai/services/playback_service.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/lyrics_parser.dart';
import 'package:tawai/utils/settings.dart';

class LyricsView extends StatefulWidget {
  const LyricsView({super.key});

  @override
  State<LyricsView> createState() => _LyricsViewState();
}

class _LyricsViewState extends State<LyricsView> {
  final ScrollController _scrollController = ScrollController();

  ParsedLyrics? _parsedLyrics;
  String? _previousTrackId;
  bool _isLoading = false;
  String? _error;
  int _currentLineIndex = -1;

  @override
  void initState() {
    super.initState();
    PlaybackService.instance.queue.addListener(_onTrackChanged);
    PlaybackService.instance.position.addListener(_onPositionChanged);
    _loadLyrics();
  }

  @override
  void dispose() {
    PlaybackService.instance.queue.removeListener(_onTrackChanged);
    PlaybackService.instance.position.removeListener(_onPositionChanged);
    _scrollController.dispose();
    super.dispose();
  }

  void _onTrackChanged() {
    final track = _currentTrack;
    if (track?.id != _previousTrackId) {
      _loadLyrics();
    }
  }

  TrackInfo? get _currentTrack => PlaybackService.instance.currentTrack;

  void _onPositionChanged() {
    if (_parsedLyrics == null || !_parsedLyrics!.synced) return;
    final pos = PlaybackService.instance.position.value;
    final newIndex = ParsedLyrics.currentLineIndex(pos, _parsedLyrics!.lines);
    if (newIndex != _currentLineIndex) {
      setState(() => _currentLineIndex = newIndex);
      _scrollToIndex(newIndex);
    }
  }

  void _scrollToIndex(int index) {
    if (index < 0 || !_scrollController.hasClients) return;
    const itemHeight = 56.0;
    final viewportHeight = _scrollController.position.viewportDimension;
    final target =
        (itemHeight * index) - (viewportHeight / 2) + (itemHeight / 2);
    final clamped = target.clamp(
      0.0,
      _scrollController.position.maxScrollExtent,
    );
    _scrollController.animateTo(
      clamped,
      duration: const Duration(milliseconds: 300),
      curve: Curves.easeOutCubic,
    );
  }

  void _loadLyrics() {
    final track = _currentTrack;
    _previousTrackId = track?.id;
    _currentLineIndex = -1;

    if (track == null) {
      setState(() {
        _parsedLyrics = null;
        _isLoading = false;
        _error = null;
      });
      return;
    }

    final stored = track.lyrics;
    if (stored != null && stored.trim().isNotEmpty) {
      final synced = ParsedLyrics.looksSynced(stored);
      setState(() {
        _parsedLyrics = ParsedLyrics.fromRaw(stored, synced: synced);
        _isLoading = false;
        _error = null;
      });
      return;
    }

    setState(() {
      _isLoading = true;
      _error = null;
    });

    _fetchFromApi(track);
  }

  Future<void> _fetchFromApi(TrackInfo track) async {
    try {
      final result = await BridgeService.instance.fetchLyrics(
        title: track.title,
        artist: track.artistsString,
        album: track.albumTitle,
        duration: track.durationSecs,
        preferSync: SettingsManager.lyricsPrefersync.value,
      );

      if (!mounted) return;
      final trackStillCurrent = _currentTrack?.id == _previousTrackId;
      if (!trackStillCurrent) return;

      if (result != null && result.lyrics.trim().isNotEmpty) {
        setState(() {
          _parsedLyrics = ParsedLyrics.fromRaw(
            result.lyrics,
            synced: result.synced,
            instrumental: result.instrumental,
          );
          _isLoading = false;
          _error = null;
        });
      } else {
        setState(() {
          _parsedLyrics = ParsedLyrics.fromRaw(
            '',
            instrumental: result?.instrumental ?? false,
          );
          _isLoading = false;
          _error = null;
        });
      }
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _isLoading = false;
        _error = e.toString();
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;

    return ValueListenableBuilder(
      valueListenable: PlaybackService.instance.queue,
      builder: (context, q, _) {
        final current = q.currentTrack;
        if (current == null) return _buildEmpty('No track playing');

        if (_isLoading) {
          return const Center(child: CircularProgressIndicator());
        }

        if (_error != null) {
          return _buildError(textTheme);
        }

        final parsed = _parsedLyrics;
        if (parsed == null || parsed.lines.isEmpty) {
          if (parsed?.instrumental == true) {
            return _buildEmpty('Instrumental');
          }
          return _buildEmpty('No lyrics found');
        }

        if (parsed.synced) {
          return _buildSynced(parsed, textTheme);
        }

        return _buildUnsynced(parsed, textTheme);
      },
    );
  }

  Widget _buildEmpty(String message) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            Icons.lyrics_outlined,
            size: AppTheme.iconXL * AppTheme.iconScale(context),
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
          SizedBox(height: AppTheme.spaceSM * 2 * AppTheme.spaceScale(context)),
          Text(message, style: Theme.of(context).textTheme.bodyLarge),
        ],
      ),
    );
  }

  Widget _buildError(TextTheme textTheme) {
    return Center(
      child: Padding(
        padding: EdgeInsets.all(
          AppTheme.spaceXL * AppTheme.spaceScale(context),
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.error_outline,
              size: AppTheme.iconXL * AppTheme.iconScale(context),
              color: Theme.of(context).colorScheme.error,
            ),
            SizedBox(
              height: AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
            ),
            Text(
              _error!,
              style: textTheme.bodyMedium,
              textAlign: TextAlign.center,
            ),
            SizedBox(
              height: AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
            ),
            FilledButton.tonalIcon(
              onPressed: _loadLyrics,
              icon: const Icon(Icons.refresh),
              label: const Text('Retry'),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildSynced(ParsedLyrics parsed, TextTheme textTheme) {
    final colors = Theme.of(context).colorScheme;

    return ListView.builder(
      controller: _scrollController,
      itemExtent: 56,
      padding: const EdgeInsets.symmetric(vertical: 24),
      itemCount: parsed.lines.length,
      itemBuilder: (context, index) {
        final isCurrent = index == _currentLineIndex;
        final isPast = index < _currentLineIndex;
        final ts = parsed.lines[index].timestamp;

        return InkWell(
          onTap: ts != null ? () => PlaybackService.instance.seek(ts) : null,
          highlightColor: Colors.transparent,
          splashFactory: NoSplash.splashFactory,
          child: AnimatedDefaultTextStyle(
            duration: const Duration(milliseconds: 200),
            style: TextStyle(
              fontSize: isCurrent
                  ? 20
                  : isPast
                  ? 14
                  : 16,
              fontWeight: isCurrent ? FontWeight.bold : FontWeight.normal,
              color: isCurrent
                  ? colors.primary
                  : isPast
                  ? colors.onSurface.withValues(alpha: 0.35)
                  : colors.onSurface,
            ),
            child: Text(parsed.lines[index].text, textAlign: TextAlign.center),
          ),
        );
      },
    );
  }

  Widget _buildUnsynced(ParsedLyrics parsed, TextTheme textTheme) {
    final text = parsed.lines.map((l) => l.text).join('\n');
    return SingleChildScrollView(
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 32),
      child: SelectableText(
        text,
        style: textTheme.bodyLarge?.copyWith(height: 1.8),
        textAlign: TextAlign.center,
      ),
    );
  }
}
