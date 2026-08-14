import 'dart:typed_data';
import 'package:flutter/material.dart';
import 'package:tawai/utils/image_cache.dart';

class CoverImage extends StatefulWidget {
  const CoverImage({
    super.key,
    this.albumId,
    this.trackId,
    this.width,
    this.height,
    this.fit = BoxFit.cover,
    this.iconSize = 48,
  });

  final String? albumId;
  final String? trackId;
  final double? width;
  final double? height;
  final BoxFit fit;
  final double iconSize;

  @override
  State<CoverImage> createState() => _CoverImageState();
}

class _CoverImageState extends State<CoverImage> {
  Uint8List? _bytes;
  bool _loading = true;
  int _generation = 0;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void didUpdateWidget(CoverImage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.albumId != widget.albumId ||
        oldWidget.trackId != widget.trackId) {
      _load();
    }
  }

  Future<void> _load() async {
    final gen = ++_generation;
    final cached = AppImageCache.instance.peek(
      albumId: widget.albumId,
      trackId: widget.trackId,
    );
    if (cached != null) {
      setState(() {
        _bytes = cached;
        _loading = false;
      });
      return;
    }
    setState(() => _loading = true);
    final bytes = await AppImageCache.instance.getCover(
      albumId: widget.albumId,
      trackId: widget.trackId,
    );
    if (!mounted || gen != _generation) return;
    setState(() {
      _bytes = bytes;
      _loading = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    if (_loading) {
      return _placeholder(context);
    }
    if (_bytes != null) {
      return Image.memory(
        _bytes!,
        fit: widget.fit,
        width: widget.width,
        height: widget.height,
        errorBuilder: (_, _, _) => _placeholder(context),
      );
    }
    return _placeholder(context);
  }

  Widget _placeholder(BuildContext context) {
    return Container(
      color: Theme.of(context).colorScheme.surfaceContainerHighest,
      child: Center(child: Icon(Icons.album, size: widget.iconSize)),
    );
  }
}
