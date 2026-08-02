import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/utils/bridge_service.dart';

class SearchLyricsDialog extends StatefulWidget {
  final String initialQuery;
  final ValueChanged<String> onSelect;

  const SearchLyricsDialog({
    super.key,
    required this.initialQuery,
    required this.onSelect,
  });

  @override
  State<SearchLyricsDialog> createState() => _SearchLyricsDialogState();
}

class _SearchLyricsDialogState extends State<SearchLyricsDialog> {
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
    } catch (_) {}
    if (mounted) setState(() => _loading = false);
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    return AlertDialog(
      title: const Text('Search Lyrics'),
      content: SizedBox(
        width: 500,
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
                          horizontal: 12, vertical: 10),
                    ),
                    onSubmitted: (_) => _doSearch(),
                  ),
                ),
                const SizedBox(width: 8),
                IconButton(
                  onPressed: _loading ? null : _doSearch,
                  icon: _loading
                      ? const SizedBox(
                          width: 18,
                          height: 18,
                          child: CircularProgressIndicator(strokeWidth: 2))
                      : const Icon(Icons.search),
                ),
              ],
            ),
            const SizedBox(height: 12),
            if (_results.isEmpty && !_loading)
              Padding(
                padding: const EdgeInsets.symmetric(vertical: 24),
                child: Text('No results found',
                    style: textTheme.bodyMedium
                        ?.copyWith(color: colors.onSurfaceVariant)),
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
                      title: Text('${r.title} — ${r.artist}',
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis),
                      subtitle: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(r.album,
                              style: textTheme.bodySmall?.copyWith(
                                  color: colors.onSurfaceVariant),
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis),
                          const SizedBox(height: 2),
                          Text(preview,
                              style: textTheme.bodySmall?.copyWith(
                                  fontStyle: FontStyle.italic,
                                  color: colors.onSurfaceVariant)),
                        ],
                      ),
                      trailing: Chip(
                        avatar: Icon(
                          r.synced ? Icons.timer : Icons.text_fields,
                          size: 14,
                        ),
                        label: Text(r.synced ? 'Synced' : 'Plain',
                            style: const TextStyle(fontSize: 11)),
                        visualDensity: VisualDensity.compact,
                      ),
                      onTap: () {
                        Navigator.of(context).pop();
                        widget.onSelect(r.lyrics);
                      },
                    );
                  },
                ),
              ),
          ],
        ),
      ),
    );
  }
}