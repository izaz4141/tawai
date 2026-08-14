import 'dart:typed_data';

import 'package:flutter/material.dart';

import 'package:tawai/utils/image_cache.dart';

class CachedNetworkImage extends StatefulWidget {
  const CachedNetworkImage({
    super.key,
    required this.url,
    this.width,
    this.height,
    this.fit = BoxFit.cover,
    this.placeholder = const SizedBox.shrink(),
  });

  final String url;
  final double? width;
  final double? height;
  final BoxFit fit;
  final Widget placeholder;

  @override
  State<CachedNetworkImage> createState() => _CachedNetworkImageState();
}

class _CachedNetworkImageState extends State<CachedNetworkImage> {
  Uint8List? _bytes;
  bool _loading = true;
  int _generation = 0;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void didUpdateWidget(CachedNetworkImage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.url != widget.url) {
      _load();
    }
  }

  Future<void> _load() async {
    final gen = ++_generation;
    final cached = AppImageCache.instance.peekUrl(widget.url);
    if (cached != null) {
      setState(() {
        _bytes = cached;
        _loading = false;
      });
      return;
    }
    if (mounted) setState(() => _loading = true);
    final bytes = await AppImageCache.instance.getUrl(widget.url);
    if (!mounted || gen != _generation) return;
    setState(() {
      _bytes = bytes;
      _loading = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    final bytes = _bytes;
    if (bytes == null || _loading) {
      return widget.placeholder;
    }
    return Image.memory(
      bytes,
      width: widget.width,
      height: widget.height,
      fit: widget.fit,
      errorBuilder: (_, _, _) => widget.placeholder,
    );
  }
}
