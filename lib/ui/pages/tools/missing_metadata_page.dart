import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/dialog/tag_editor.dart';
import 'package:tawai/utils/bridge_service.dart';

class MissingMetadataPage extends StatefulWidget {
  const MissingMetadataPage({super.key});

  @override
  State<MissingMetadataPage> createState() => _MissingMetadataPageState();
}

class _MissingMetadataPageState extends State<MissingMetadataPage> {
  bool _checkTitle = true;
  bool _checkArtist = true;
  bool _checkAlbum = true;
  bool _checkGenre = true;
  bool _checkYear = true;
  bool _checkTrackNumber = true;
  bool _checkCover = true;

  List<MissingMetadataEntry> _results = [];
  bool _scanning = false;
  String? _error;

  Future<void> _scan() async {
    setState(() {
      _scanning = true;
      _error = null;
      _results = [];
    });

    try {
      final response = await BridgeService.instance.findMissingMetadata(
        checkTitle: _checkTitle,
        checkArtist: _checkArtist,
        checkAlbum: _checkAlbum,
        checkGenre: _checkGenre,
        checkYear: _checkYear,
        checkTrackNumber: _checkTrackNumber,
        checkCover: _checkCover,
      );
      if (!mounted) return;
      setState(() {
        _results = response?.tracks ?? [];
        _scanning = false;
      });
    } catch (e) {
      if (mounted) {
        setState(() {
          _error = e.toString();
          _scanning = false;
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
          'Missing Metadata',
          style: isDesktop ? textTheme.titleLarge : null,
        ),
      ),
      body: Column(
        children: [
          Container(
            padding: EdgeInsets.all(
              AppTheme.spaceMD * AppTheme.spaceScale(context),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('Check for tracks missing:', style: textTheme.titleSmall),
                SizedBox(height: AppTheme.spaceSM),
                Wrap(
                  spacing: AppTheme.spaceSM,
                  runSpacing: AppTheme.spaceXS,
                  children: [
                    _buildCheckChip('Title', _checkTitle, (v) {
                      setState(() => _checkTitle = v);
                    }),
                    _buildCheckChip('Artist', _checkArtist, (v) {
                      setState(() => _checkArtist = v);
                    }),
                    _buildCheckChip('Album', _checkAlbum, (v) {
                      setState(() => _checkAlbum = v);
                    }),
                    _buildCheckChip('Genre', _checkGenre, (v) {
                      setState(() => _checkGenre = v);
                    }),
                    _buildCheckChip('Year', _checkYear, (v) {
                      setState(() => _checkYear = v);
                    }),
                    _buildCheckChip('Track #', _checkTrackNumber, (v) {
                      setState(() => _checkTrackNumber = v);
                    }),
                    _buildCheckChip('Cover', _checkCover, (v) {
                      setState(() => _checkCover = v);
                    }),
                  ],
                ),
                SizedBox(height: AppTheme.spaceSM),
                SizedBox(
                  width: double.infinity,
                  child: FilledButton.icon(
                    onPressed: _scanning ? null : _scan,
                    icon: _scanning
                        ? SizedBox(
                            width: 16,
                            height: 16,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.search),
                    label: Text(
                      _scanning ? 'Scanning...' : 'Find Missing Metadata',
                    ),
                  ),
                ),
              ],
            ),
          ),
          Divider(height: 1),
          if (_error != null)
            Padding(
              padding: EdgeInsets.all(AppTheme.spaceMD),
              child: Text(
                _error!,
                style: textTheme.bodySmall?.copyWith(color: colors.error),
              ),
            ),
          Expanded(
            child: _results.isEmpty
                ? Center(
                    child: Text(
                      _scanning ? '' : 'No results. Tap scan to check.',
                      style: textTheme.bodyLarge,
                    ),
                  )
                : ListView.builder(
                    itemCount: _results.length,
                    itemBuilder: (context, index) {
                      final entry = _results[index];
                      return Card(
                        margin: EdgeInsets.symmetric(
                          horizontal: AppTheme.spaceSM,
                          vertical: AppTheme.spaceXS,
                        ),
                        child: ListTile(
                          leading: Icon(
                            Icons.warning_amber_rounded,
                            color: colors.error,
                          ),
                          title: Text(
                            entry.filePath.split('/').last,
                            style: textTheme.bodyMedium,
                            overflow: TextOverflow.ellipsis,
                          ),
                          subtitle: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                entry.filePath,
                                style: textTheme.bodySmall,
                                overflow: TextOverflow.ellipsis,
                              ),
                              SizedBox(height: AppTheme.spaceXS),
                              Wrap(
                                spacing: AppTheme.spaceXS,
                                children: entry.missingFields
                                    .map(
                                      (f) => Container(
                                        padding: EdgeInsets.symmetric(
                                          horizontal: 6,
                                          vertical: 1,
                                        ),
                                        decoration: BoxDecoration(
                                          color: colors.errorContainer,
                                          borderRadius: BorderRadius.circular(
                                            4,
                                          ),
                                        ),
                                        child: Text(
                                          f,
                                          style: textTheme.labelSmall?.copyWith(
                                            color: colors.onErrorContainer,
                                          ),
                                        ),
                                      ),
                                    )
                                    .toList(),
                              ),
                            ],
                          ),
                          trailing: IconButton(
                            icon: const Icon(Icons.edit),
                            tooltip: 'Edit tags',
                            onPressed: () => showTagEditorDialog(
                              context,
                              initialPath: entry.filePath,
                            ),
                          ),
                        ),
                      );
                    },
                  ),
          ),
        ],
      ),
    );
  }

  Widget _buildCheckChip(
    String label,
    bool selected,
    ValueChanged<bool> onChanged,
  ) {
    return FilterChip(
      label: Text(label, style: const TextStyle(fontSize: 12)),
      selected: selected,
      onSelected: onChanged,
      visualDensity: VisualDensity.compact,
    );
  }
}
