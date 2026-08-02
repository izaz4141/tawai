import 'package:flutter/material.dart';
import 'package:tawai/ui/theme/app_theme.dart';

class _TooltipData {
  final String letter;
  final double globalTop;
  _TooltipData(this.letter, this.globalTop);
}

class AlphabetIndexScroller<T> extends StatefulWidget {
  static const double kStripWidth = 28;

  final ScrollController scrollController;
  final List<T> items;
  final String Function(T) labelSelector;
  final double itemExtent;
  final int? gridColumns;
  final int startIndex;

  const AlphabetIndexScroller({
    super.key,
    required this.scrollController,
    required this.items,
    required this.labelSelector,
    required this.itemExtent,
    this.gridColumns,
    this.startIndex = 0,
  });

  @override
  State<AlphabetIndexScroller<T>> createState() =>
      _AlphabetIndexScrollerState<T>();
}

class _AlphabetIndexScrollerState<T>
    extends State<AlphabetIndexScroller<T>> {
  final _scrollerKey = GlobalKey();
  final _tooltipNotifier = ValueNotifier<_TooltipData?>(null);
  OverlayEntry? _overlayEntry;
  String? _activeLetter;
  String? _currentLetter;
  bool _isDragging = false;
  bool _isTap = false;

  @override
  void initState() {
    super.initState();
    widget.scrollController.addListener(_onScroll);
    _onScroll();
  }

  @override
  void dispose() {
    widget.scrollController.removeListener(_onScroll);
    _hideTooltip();
    _tooltipNotifier.dispose();
    super.dispose();
  }

  List<String> get _letters {
    final set = <String>{};
    for (final item in widget.items) {
      final label = widget.labelSelector(item);
      if (label.isEmpty) continue;
      final char = label[0].toUpperCase();
      set.add(RegExp(r'^[A-Z]$').hasMatch(char) ? char : '#');
    }
    final letters = set.toList()..sort((a, b) {
      if (a == '#') return 1;
      if (b == '#') return -1;
      return a.compareTo(b);
    });
    return letters;
  }

  Map<String, int> get _letterIndexMap {
    final map = <String, int>{};
    for (int i = 0; i < widget.items.length; i++) {
      final label = widget.labelSelector(widget.items[i]);
      if (label.isEmpty) continue;
      final char = label[0].toUpperCase();
      final key = RegExp(r'^[A-Z]$').hasMatch(char) ? char : '#';
      map.putIfAbsent(key, () => i);
    }
    return map;
  }

  double _offsetForLetter(String letter) {
    final index = _letterIndexMap[letter] ?? 0;
    final columns = widget.gridColumns ?? 1;
    final row = index ~/ columns;
    return (row + widget.startIndex) * widget.itemExtent;
  }

  void _scrollToLetter(String letter) {
    final offset = _offsetForLetter(letter);
    if (widget.scrollController.hasClients) {
      widget.scrollController.animateTo(
        offset,
        duration: const Duration(milliseconds: 200),
        curve: Curves.easeOut,
      );
    }
  }

  String? _letterFromDy(double dy, double stripHeight) {
    if (widget.items.isEmpty) return null;
    final letters = _letters;
    if (letters.isEmpty) return null;
    final itemHeight = stripHeight / letters.length;
    final index = (dy / itemHeight).floor().clamp(0, letters.length - 1);
    return letters[index];
  }

  void _showTooltip(String letter, double stripHeight) {
    final letters = _letters;
    if (letters.isEmpty) return;
    final itemHeight = stripHeight / letters.length;
    final index = letters.indexOf(letter);
    final localCenter = index * itemHeight + itemHeight / 2;

    final renderBox =
        _scrollerKey.currentContext?.findRenderObject() as RenderBox?;
    if (renderBox == null) return;
    final globalOffset = renderBox.localToGlobal(Offset.zero);
    final globalTop = globalOffset.dy + localCenter - 32;

    if (_overlayEntry == null) {
      final colors = Theme.of(context).colorScheme;
      _overlayEntry = OverlayEntry(
        builder: (_) => ValueListenableBuilder<_TooltipData?>(
          valueListenable: _tooltipNotifier,
          builder: (context, data, _) {
            if (data == null) return const SizedBox.shrink();
            return Positioned(
              right: 40,
              top: data.globalTop,
              child: Container(
                width: 64,
                height: 64,
                decoration: BoxDecoration(
                  color: colors.primaryContainer,
                  borderRadius: BorderRadius.circular(AppTheme.radiusLG),
                ),
                alignment: Alignment.center,
                child: Text(
                  data.letter,
                  style: TextStyle(
                    fontSize: 28,
                    fontWeight: FontWeight.bold,
                    color: colors.onPrimaryContainer,
                  ),
                ),
              ),
            );
          },
        ),
      );
      Overlay.of(context, rootOverlay: true).insert(_overlayEntry!);
    }
    _tooltipNotifier.value = _TooltipData(letter, globalTop);
  }

  void _hideTooltip() {
    _tooltipNotifier.value = null;
    _overlayEntry?.remove();
    _overlayEntry = null;
  }

  void _onDragStart(double dy, double stripHeight) {
    final letter = _letterFromDy(dy, stripHeight);
    if (letter != null) {
      setState(() {
        _activeLetter = letter;
        _isDragging = true;
      });
      _showTooltip(letter, stripHeight);
      _scrollToLetter(letter);
    }
  }

  void _onDragUpdate(double dy, double stripHeight) {
    final letter = _letterFromDy(dy, stripHeight);
    if (letter != null && letter != _activeLetter) {
      setState(() => _activeLetter = letter);
      _showTooltip(letter, stripHeight);
      _scrollToLetter(letter);
    }
  }

  void _onScroll() {
    if (!widget.scrollController.hasClients) return;
    final columns = widget.gridColumns ?? 1;
    final offset = widget.scrollController.offset;
    final row = (offset / widget.itemExtent).floor();
    final index = row * columns - widget.startIndex;
    if (index < 0 || index >= widget.items.length) {
      _updateCurrentLetter(null);
      return;
    }
    final label = widget.labelSelector(widget.items[index]);
    final char = label.isEmpty ? null : label[0].toUpperCase();
    final letter = char != null && RegExp(r'^[A-Z]$').hasMatch(char) ? char : '#';
    _updateCurrentLetter(letter);
  }

  void _updateCurrentLetter(String? letter) {
    if (letter != _currentLetter) {
      setState(() => _currentLetter = letter);
    }
  }

  void _onDragEnd() {
    setState(() {
      _isDragging = false;
      _activeLetter = null;
    });
    _hideTooltip();
    _onScroll();
  }

  @override
  Widget build(BuildContext context) {
    final letters = _letters;
    if (letters.isEmpty) return const SizedBox.shrink();

    final colors = Theme.of(context).colorScheme;

    return LayoutBuilder(
      builder: (context, constraints) {
        final stripHeight = constraints.maxHeight;
        final activeIndex = _activeLetter != null
            ? letters.indexOf(_activeLetter!)
            : -1;
        final currentIndex = _currentLetter != null
            ? letters.indexOf(_currentLetter!)
            : -1;

        return Listener(
          key: _scrollerKey,
          onPointerDown: (event) {
            _isTap = true;
            _onDragStart(event.localPosition.dy, stripHeight);
          },
          onPointerMove: (event) {
            if (_isTap) _isTap = false;
            _onDragUpdate(event.localPosition.dy, stripHeight);
          },
          onPointerUp: (event) {
            if (_isTap) {
              final letter = _letterFromDy(event.localPosition.dy, stripHeight);
              if (letter != null) {
                _scrollToLetter(letter);
              }
            }
            _onDragEnd();
          },
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 8),
            child: Material(
              elevation: 2,
              borderRadius: BorderRadius.circular(AppTheme.radiusSM),
              color: colors.surfaceContainerHighest.withValues(alpha: 0.9),
              child: Column(
                children: List.generate(letters.length, (i) {
                  final hilite = (i == activeIndex && _isDragging) || i == currentIndex;
                  return Expanded(
                    child: Center(
                      child: Text(
                        letters[i],
                        style: TextStyle(
                          fontSize: 10,
                          fontWeight: FontWeight.w600,
                          color: hilite
                              ? colors.primary
                              : colors.onSurfaceVariant,
                        ),
                      ),
                    ),
                  );
                }),
              ),
            ),
          ),
        );
      },
    );
  }
}
