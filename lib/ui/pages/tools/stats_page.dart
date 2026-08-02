import 'dart:async';
import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/section_header.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/settings.dart';

class StatsPage extends StatefulWidget {
  const StatsPage({super.key});

  @override
  State<StatsPage> createState() => _StatsPageState();
}

class _StatsPageState extends State<StatsPage> with SingleTickerProviderStateMixin {
  LibraryStats? _stats;
  bool _loading = true;
  String? _error;
  late AnimationController _animController;
  late Animation<double> _fadeAnim;

  @override
  void initState() {
    super.initState();
    _animController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 600),
    );
    _fadeAnim = CurvedAnimation(parent: _animController, curve: Curves.easeOut);
    _load();
  }

  @override
  void dispose() {
    _animController.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final response = await BridgeService.instance.getLibraryStats(
        namingPattern: SettingsManager.namingPattern.value,
      );
      if (!mounted) return;
      if (response?.stats != null) {
        setState(() {
          _stats = response!.stats;
          _loading = false;
        });
        _animController.forward(from: 0);
      } else {
        setState(() {
          _error = response?.error ?? 'Failed to load stats';
          _loading = false;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _error = e.toString();
          _loading = false;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final colors = Theme.of(context).colorScheme;
    final isDesktop = AppTheme.isDesktop(context);

    return Scaffold(
      appBar: AppBar(
        title: Text(
          'Collection Statistics',
          style: isDesktop ? textTheme.titleLarge : null,
        ),
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _error != null
          ? Center(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(Icons.error_outline, size: 48, color: colors.error),
                  const SizedBox(height: 16),
                  Text(_error!, style: textTheme.bodyLarge),
                  const SizedBox(height: 16),
                  FilledButton.icon(
                    onPressed: _load,
                    icon: const Icon(Icons.refresh),
                    label: const Text('Retry'),
                  ),
                ],
              ),
            )
          : _stats != null
          ? _buildContent(context, isDesktop, textTheme, colors)
          : const SizedBox.shrink(),
    );
  }

  Widget _buildContent(
    BuildContext context,
    bool isDesktop,
    TextTheme textTheme,
    ColorScheme colors,
  ) {
    final s = _stats!;

    final sizeCards = <_StatCard>[
      _StatCard(icon: Icons.audiotrack, label: 'Total Tracks', value: _fmt(s.totalTracks)),
      _StatCard(icon: Icons.album, label: 'Total Albums', value: _fmt(s.totalAlbums)),
      _StatCard(icon: Icons.person, label: 'Total Artists', value: _fmt(s.totalArtists)),
      _StatCard(icon: Icons.timer, label: 'Total Duration', value: _formatDuration(Duration(seconds: s.totalDurationSecs.round()))),
      _StatCard(icon: Icons.storage, label: 'Total File Size', value: _formatSize(s.totalFileSize)),
    ];

    final avgCards = <_StatCard>[
      _StatCard(icon: Icons.speed, label: 'Avg Track Duration', value: _formatDuration(Duration(seconds: s.averageTrackDurationSecs.round()))),
      _StatCard(icon: Icons.album, label: 'Tracks / Album', value: s.tracksPerAlbumAvg.toStringAsFixed(1)),
      _StatCard(icon: Icons.people, label: 'Tracks / Artist', value: s.tracksPerArtistAvg.toStringAsFixed(1)),
    ];

    final highlightPairs = <List<_StatCard>>[];
    final highlights = <_StatCard>[];
    if (s.largestAlbumTitle != null) {
      highlights.add(_StatCard(icon: Icons.photo_library, label: 'Largest Album', value: s.largestAlbumTitle!, subtitle: '${_fmt(s.largestAlbumTracks)} tracks'));
    }
    if (s.mostProlificArtist != null) {
      highlights.add(_StatCard(icon: Icons.star, label: 'Most Prolific', value: s.mostProlificArtist!, subtitle: '${_fmt(s.mostProlificArtistTracks)} tracks'));
    }
    if (s.mostCommonGenre != null) {
      highlights.add(_StatCard(icon: Icons.tag_rounded, label: 'Top Genre', value: s.mostCommonGenre!, subtitle: '${_fmt(s.genreCount)} tracks'));
    }
    if (s.shortestTrackTitle != null) {
      highlights.add(_StatCard(icon: Icons.timer_off, label: 'Shortest Track', value: s.shortestTrackTitle!, subtitle: _formatDuration(Duration(seconds: s.shortestTrackDuration!.round()))));
    }
    if (s.longestTrackTitle != null) {
      highlights.add(_StatCard(icon: Icons.timer, label: 'Longest Track', value: s.longestTrackTitle!, subtitle: _formatDuration(Duration(seconds: s.longestTrackDuration!.round()))));
    }
    if (s.oldestYear != null) {
      highlights.add(_StatCard(icon: Icons.calendar_today, label: 'Oldest Year', value: s.oldestYear!));
    }
    if (s.newestYear != null) {
      highlights.add(_StatCard(icon: Icons.calendar_today, label: 'Newest Year', value: s.newestYear!));
    }
    for (int i = 0; i < highlights.length; i += 2) {
      highlightPairs.add(highlights.sublist(i, (i + 2 > highlights.length) ? i + 1 : i + 2));
    }

    final qualityCards = <_StatCard>[
      if (s.averageBitrate != null)
        _StatCard(icon: Icons.speed, label: 'Avg Bitrate', value: '${s.averageBitrate!.round()} kbps'),
      _StatCard(icon: Icons.image, label: 'Tracks With Cover', value: '${_fmt(s.tracksWithCover)} / ${_fmt(s.tracksWithCover + s.tracksWithoutCover)}'),
      _StatCard(icon: Icons.lyrics, label: 'Tracks With Lyrics', value: '${_fmt(s.tracksWithLyrics)} / ${_fmt(s.tracksWithLyrics + s.tracksWithoutLyrics)}'),
      _StatCard(icon: Icons.music_note, label: 'Tracks With MBID', value: '${_fmt(s.tracksWithMbid)} / ${_fmt(s.totalTracks)}'),
    ];

    final maxFormatCount = s.formatBreakdown.isEmpty ? 1 : s.formatBreakdown.map((e) => e.count).reduce((a, b) => a > b ? a : b);
    final maxDecadeCount = s.decadeDistribution.isEmpty ? 1 : s.decadeDistribution.map((e) => e.count).reduce((a, b) => a > b ? a : b);

    return RefreshIndicator(
      onRefresh: _load,
      child: FadeTransition(
        opacity: _fadeAnim,
        child: ListView(
          padding: EdgeInsets.all(AppTheme.spaceLG * AppTheme.spaceScale(context)),
          children: [
            _buildSummaryBanner(s, colors, textTheme),
            SizedBox(height: AppTheme.spaceXL),
            SectionHeader(leading: Icon(Icons.bar_chart, color: colors.onPrimaryContainer), title: 'Library Size'),
            _buildStatGrid(sizeCards, isDesktop),
            SizedBox(height: AppTheme.spaceLG),
            SectionHeader(leading: Icon(Icons.analytics, color: colors.onPrimaryContainer), title: 'Averages'),
            _buildStatGrid(avgCards, isDesktop),
            if (highlightPairs.isNotEmpty) ...[
              SizedBox(height: AppTheme.spaceLG),
              SectionHeader(leading: Icon(Icons.emoji_events, color: colors.onPrimaryContainer), title: 'Highlights'),
              _buildPairsGrid(highlightPairs),
            ],
            SizedBox(height: AppTheme.spaceLG),
            SectionHeader(leading: Icon(Icons.verified, color: colors.onPrimaryContainer), title: 'Quality'),
            _buildStatGrid(qualityCards, isDesktop),
            if (s.namingConformityPct != null) ...[
              SizedBox(height: AppTheme.spaceSM),
              _buildNamingConformityCard(s.namingConformityPct!, colors, textTheme),
            ],
            SizedBox(height: AppTheme.spaceXL),
            SectionHeader(leading: Icon(Icons.insert_drive_file, color: colors.onPrimaryContainer), title: 'Format Breakdown'),
            SizedBox(height: AppTheme.spaceSM),
            Card(
              child: Padding(
                padding: EdgeInsets.all(AppTheme.spaceLG),
                child: Column(
                  children: s.formatBreakdown.map((f) => _buildBarRow('.${f.format}', f.count, maxFormatCount, colors, textTheme)).toList(),
                ),
              ),
            ),
            if (s.decadeDistribution.isNotEmpty) ...[
              SizedBox(height: AppTheme.spaceLG),
              SectionHeader(leading: Icon(Icons.date_range, color: colors.onPrimaryContainer), title: 'Decade Distribution'),
              SizedBox(height: AppTheme.spaceSM),
              Card(
                child: Padding(
                  padding: EdgeInsets.all(AppTheme.spaceLG),
                  child: Column(
                    children: s.decadeDistribution.map((d) => _buildBarRow(d.decade, d.count, maxDecadeCount, colors, textTheme)).toList(),
                  ),
                ),
              ),
            ],
            SizedBox(height: 120),
          ],
        ),
      ),
    );
  }

  Widget _buildSummaryBanner(LibraryStats s, ColorScheme colors, TextTheme textTheme) {
    final duration = Duration(seconds: s.totalDurationSecs.round());
    return Card(
      child: Padding(
        padding: EdgeInsets.all(AppTheme.spaceLG * AppTheme.spaceScale(context)),
        child: Row(
          children: [
            Icon(Icons.analytics_outlined, size: AppTheme.iconXL, color: colors.primary),
            SizedBox(width: AppTheme.spaceMD),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('Collection Overview', style: textTheme.titleMedium),
                  SizedBox(height: AppTheme.spaceXS),
                  Text('${_fmt(s.totalTracks)} tracks  \u2022  ${_formatSize(s.totalFileSize)}  \u2022  ${_formatDuration(duration)}',
                    style: textTheme.bodyMedium?.copyWith(color: colors.onSurfaceVariant)),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildStatGrid(List<_StatCard> cards, bool isDesktop) {
    if (isDesktop) {
      return Wrap(
        spacing: AppTheme.spaceMD,
        runSpacing: AppTheme.spaceMD,
        children: cards.map((c) => SizedBox(width: 240, child: c)).toList(),
      );
    }
    return Column(
      children: cards.map((c) => Padding(
        padding: EdgeInsets.only(bottom: AppTheme.spaceSM),
        child: c,
      )).toList(),
    );
  }

  Widget _buildPairsGrid(List<List<_StatCard>> pairs) {
    return Column(
      children: pairs.map((pair) {
        return Padding(
          padding: EdgeInsets.only(bottom: AppTheme.spaceSM),
          child: Row(
            children: [
              for (final c in pair) Expanded(child: c),
              if (pair.length == 1) const Spacer(),
            ],
          ),
        );
      }).toList(),
    );
  }

  Widget _buildNamingConformityCard(double pct, ColorScheme colors, TextTheme textTheme) {
    final barColor = pct >= 80 ? Colors.green
        : pct >= 50 ? Colors.orange
        : Colors.red;
    return Card(
      child: Padding(
        padding: EdgeInsets.all(AppTheme.spaceLG),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(Icons.checklist, size: AppTheme.iconMD, color: colors.primary),
                SizedBox(width: AppTheme.spaceMD),
                Text('Naming Convention', style: textTheme.titleMedium),
                const Spacer(),
                Text('${pct.toStringAsFixed(1)}%', style: textTheme.titleMedium?.copyWith(color: barColor, fontWeight: FontWeight.w600)),
              ],
            ),
            SizedBox(height: AppTheme.spaceSM),
            ClipRRect(
              borderRadius: BorderRadius.circular(AppTheme.radiusSM),
              child: LinearProgressIndicator(
                value: pct / 100,
                minHeight: 8,
                backgroundColor: colors.surfaceContainerHighest,
                valueColor: AlwaysStoppedAnimation(barColor),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildBarRow(String label, int count, int maxCount, ColorScheme colors, TextTheme textTheme) {
    final pct = maxCount > 0 ? count / maxCount : 0.0;
    return Padding(
      padding: EdgeInsets.symmetric(vertical: AppTheme.spaceXS),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Text(label, style: textTheme.bodyMedium),
              const Spacer(),
              Text(_fmt(count), style: textTheme.bodyMedium?.copyWith(color: colors.onSurfaceVariant)),
            ],
          ),
          SizedBox(height: AppTheme.spaceXS),
          ClipRRect(
            borderRadius: BorderRadius.circular(AppTheme.radiusSM),
            child: LinearProgressIndicator(
              value: pct,
              minHeight: 6,
              backgroundColor: colors.surfaceContainerHighest,
              valueColor: AlwaysStoppedAnimation(colors.primary),
            ),
          ),
        ],
      ),
    );
  }

  String _fmt(int n) {
    if (n >= 1000000) return '${(n / 1000000).toStringAsFixed(1)}M';
    if (n >= 1000) return '${(n / 1000).toStringAsFixed(1)}K';
    return n.toString();
  }

  String _formatDuration(Duration d) {
    final hours = d.inHours;
    final minutes = d.inMinutes.remainder(60);
    final secs = d.inSeconds.remainder(60);
    if (hours > 0) return '${hours}h ${minutes}m';
    if (minutes > 0) return '${minutes}m ${secs}s';
    return '${secs}s';
  }

  String _formatSize(int bytes) {
    if (bytes >= 1073741824) return '${(bytes / 1073741824).toStringAsFixed(1)} GB';
    if (bytes >= 1048576) return '${(bytes / 1048576).toStringAsFixed(1)} MB';
    if (bytes >= 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    return '$bytes B';
  }
}

class _StatCard extends StatelessWidget {
  final IconData icon;
  final String label;
  final String value;
  final String? subtitle;

  const _StatCard({
    required this.icon,
    required this.label,
    required this.value,
    this.subtitle,
  });

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    return Card(
      clipBehavior: Clip.antiAlias,
      child: Padding(
        padding: const EdgeInsets.all(AppTheme.spaceLG),
        child: Row(
          crossAxisAlignment: subtitle != null ? CrossAxisAlignment.start : CrossAxisAlignment.center,
          children: [
            Container(
              width: 44,
              height: 44,
              decoration: BoxDecoration(
                color: colors.primaryContainer,
                borderRadius: BorderRadius.circular(AppTheme.radiusSM),
              ),
              child: Icon(icon, size: AppTheme.iconMD, color: colors.onPrimaryContainer),
            ),
            SizedBox(width: AppTheme.spaceMD),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(label, style: textTheme.bodySmall?.copyWith(color: colors.onSurfaceVariant)),
                  SizedBox(height: AppTheme.spaceXS),
                  Text(value, style: textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w600)),
                  if (subtitle != null) ...[
                    SizedBox(height: AppTheme.spaceXS),
                    Text(subtitle!, style: textTheme.bodySmall?.copyWith(color: colors.onSurfaceVariant)),
                  ],
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
