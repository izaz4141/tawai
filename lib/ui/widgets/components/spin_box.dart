import 'package:flutter/material.dart';

import 'package:tawai/ui/theme/app_theme.dart';

class SpinBox<T extends num> extends StatefulWidget {
  const SpinBox({
    super.key,
    required this.title,
    required this.subtitle,
    required this.valueListenable,
    required this.min,
    required this.max,
    this.step,
    this.defaultValue,
    this.width = AppTheme.spaceXXL * 2.5,
    this.enabled = true,
    this.decimalPlaces,
  });

  final String title;
  final String subtitle;
  final ValueNotifier<T> valueListenable;
  final T min;
  final T max;
  final T? step;
  final T? defaultValue;
  final double width;
  final bool enabled;
  final int? decimalPlaces;

  @override
  State<SpinBox<T>> createState() => _SpinBoxState<T>();
}

class _SpinBoxState<T extends num> extends State<SpinBox<T>> {
  late TextEditingController _controller;
  late FocusNode _focusNode;
  bool _isEditing = false;

  T get _step => widget.step ?? (T == int ? 1 as T : 1.0 as T);

  int get _decimalPlaces => widget.decimalPlaces ?? 2;

  String _formatValue(T value) {
    if (T == int) return value.toString();
    return (value as double).toStringAsFixed(_decimalPlaces);
  }

  T? _parseValue(String text) {
    if (T == int) {
      final v = int.tryParse(text);
      return v as T?;
    }
    final v = double.tryParse(text);
    return v as T?;
  }

  T _round(T value) {
    if (T == int) return value;
    return double.parse((value as double).toStringAsFixed(_decimalPlaces)) as T;
  }

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController(
      text: _formatValue(widget.valueListenable.value),
    );
    _focusNode = FocusNode();
    widget.valueListenable.addListener(_updateController);
    _focusNode.addListener(_handleFocusChange);
  }

  void _updateController() {
    final formatted = _formatValue(widget.valueListenable.value);
    if (!_isEditing && _controller.text != formatted) {
      _controller.text = formatted;
    }
  }

  void _handleFocusChange() {
    if (!_focusNode.hasFocus) {
      _saveValue();
      setState(() => _isEditing = false);
    } else {
      setState(() => _isEditing = true);
    }
  }

  void _saveValue() {
    final parsed = _parseValue(_controller.text);
    if (parsed != null && parsed >= widget.min && parsed <= widget.max) {
      widget.valueListenable.value = _round(parsed);
    } else {
      _controller.text = _formatValue(widget.valueListenable.value);
    }
  }

  void _increment() {
    final current = widget.valueListenable.value;
    if (current < widget.max) {
      final next = current + _step;
      widget.valueListenable.value = next > widget.max ? widget.max : _round(next as T);
    }
  }

  void _decrement() {
    final current = widget.valueListenable.value;
    if (current > widget.min) {
      final next = current - _step;
      widget.valueListenable.value = next < widget.min ? widget.min : _round(next as T);
    }
  }

  @override
  void dispose() {
    widget.valueListenable.removeListener(_updateController);
    _controller.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  void _resetToDefault() {
    if (widget.defaultValue != null) {
      widget.valueListenable.value = widget.defaultValue!;
    }
  }

  Widget _compactButton(IconData icon, VoidCallback? onPressed) {
    final iconScale = AppTheme.iconScale(context);
    return SizedBox(
      width: 36,
      height: 36,
      child: IconButton(
        icon: Icon(icon),
        iconSize: AppTheme.iconSM * iconScale,
        padding: EdgeInsets.zero,
        constraints: const BoxConstraints(),
        onPressed: onPressed,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final iconScale = AppTheme.iconScale(context);
    final spaceScale = AppTheme.spaceScale(context);

    return ListTile(
      title: Text(widget.title, style: textTheme.bodyMedium),
      subtitle: widget.subtitle.isNotEmpty
          ? Text(
              widget.subtitle,
              style: textTheme.bodySmall?.copyWith(
                color: colors.onSurfaceVariant,
              ),
            )
          : null,
      trailing: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          _compactButton(Icons.remove, widget.enabled ? _decrement : null),
          SizedBox(
            width: widget.width * spaceScale,
            child: TextField(
              controller: _controller,
              focusNode: _focusNode,
              keyboardType: TextInputType.number,
              textAlign: TextAlign.center,
              style: textTheme.bodyMedium,
              readOnly: !widget.enabled,
              decoration: InputDecoration(
                isDense: true,
                contentPadding: EdgeInsets.symmetric(
                  horizontal: AppTheme.spaceSM * spaceScale,
                  vertical: AppTheme.spaceSM * spaceScale,
                ),
                border: const OutlineInputBorder(),
              ),
              onChanged: widget.enabled
                  ? (value) {
                      final parsed = _parseValue(value);
                      if (parsed != null &&
                          parsed >= widget.min &&
                          parsed <= widget.max) {
                        widget.valueListenable.value = _round(parsed);
                      }
                    }
                  : null,
              onSubmitted: widget.enabled
                  ? (value) {
                      _saveValue();
                      _focusNode.unfocus();
                    }
                  : null,
            ),
          ),
          _compactButton(Icons.add, widget.enabled ? _increment : null),
          if (widget.defaultValue != null)
            _compactButton(
              Icons.arrow_back,
              widget.enabled ? _resetToDefault : null,
            ),
        ],
      ),
    );
  }
}
