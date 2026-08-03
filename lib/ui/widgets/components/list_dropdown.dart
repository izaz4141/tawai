import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';

import 'package:tawai/ui/theme/app_theme.dart';

class ListDropdown extends StatefulWidget {
  final String title;
  final String subtitle;
  final ValueListenable<String> valueListenable;
  final List<DropdownMenuItem<String>> items;
  final String? defaultValue;
  final bool editable;
  final bool enabled;
  final Set<String> disabledValues;
  final Function(String)? onChange;
  final List<Widget>? suffixWidgets;

  const ListDropdown({
    super.key,
    required this.title,
    required this.subtitle,
    required this.valueListenable,
    required this.items,
    this.defaultValue,
    this.editable = false,
    this.enabled = true,
    this.disabledValues = const {},
    this.onChange,
    this.suffixWidgets,
  });

  @override
  State<ListDropdown> createState() => _ListDropdownState();
}

class _ListDropdownState extends State<ListDropdown> {
  late TextEditingController _controller;

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController(text: widget.valueListenable.value);
    widget.valueListenable.addListener(_onValueListenableChanged);
  }

  @override
  void didUpdateWidget(ListDropdown oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.valueListenable != widget.valueListenable) {
      oldWidget.valueListenable.removeListener(_onValueListenableChanged);
      widget.valueListenable.addListener(_onValueListenableChanged);
      _onValueListenableChanged();
    }
  }

  @override
  void dispose() {
    widget.valueListenable.removeListener(_onValueListenableChanged);
    _controller.dispose();
    super.dispose();
  }

  void _onValueListenableChanged() {
    if (!widget.editable) return;
    final newValue = widget.valueListenable.value;
    if (_controller.text != newValue) {
      _controller.value = TextEditingValue(
        text: newValue,
        selection: TextSelection.collapsed(offset: newValue.length),
      );
    }
  }

  void _handleChange(String newValue) {
    if (widget.disabledValues.contains(newValue)) return;
    if (widget.onChange != null) {
      widget.onChange!(newValue);
    } else if (widget.valueListenable is ValueNotifier<String>) {
      (widget.valueListenable as ValueNotifier<String>).value = newValue;
    }
  }

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final colors = Theme.of(context).colorScheme;

    return ListTile(
      title: Text(widget.title, style: textTheme.bodyMedium),
      subtitle: Text(widget.subtitle, style: textTheme.bodySmall),
      trailing: SizedBox(
        width: 250 * AppTheme.spaceScale(context),
        child: ValueListenableBuilder<String>(
          valueListenable: widget.valueListenable,
          builder: (context, value, _) {
            if (widget.editable) {
              return _buildEditable(textTheme, colors);
            }
            return _buildDropdown(value, textTheme, colors);
          },
        ),
      ),
    );
  }

  Widget _buildEditable(TextTheme textTheme, ColorScheme colors) {
    return TextField(
      controller: _controller,
      style: textTheme.bodyMedium,
      decoration: InputDecoration(
        contentPadding: EdgeInsets.symmetric(
          horizontal: AppTheme.spaceSM * AppTheme.spaceScale(context),
          vertical: AppTheme.spaceSM * AppTheme.spaceScale(context),
        ),
        border: const OutlineInputBorder(
          borderRadius: BorderRadius.all(Radius.circular(AppTheme.radiusSM)),
        ),
        suffixIcon: widget.enabled
            ? Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  if (widget.suffixWidgets != null) ...widget.suffixWidgets!,
                  PopupMenuButton<String>(
                    icon: const Icon(Icons.arrow_drop_down),
                    tooltip: 'Presets',
                    onSelected: _handleChange,
                    itemBuilder: (_) => widget.items.map((e) {
                      final disabled = widget.disabledValues.contains(e.value);
                      return PopupMenuItem<String>(
                        value: e.value,
                        enabled: !disabled,
                        child: Opacity(
                          opacity: disabled ? 0.4 : 1.0,
                          child: Row(
                            children: [
                              Expanded(
                                child: e.child ?? const SizedBox.shrink(),
                              ),
                              if (disabled)
                                Text(
                                  '(unavailable)',
                                  style: TextStyle(
                                    fontSize: 10,
                                    color: Theme.of(
                                      context,
                                    ).colorScheme.onSurfaceVariant,
                                  ),
                                ),
                            ],
                          ),
                        ),
                      );
                    }).toList(),
                  ),
                  if (widget.defaultValue != null)
                    IconButton(
                      icon: Icon(Icons.arrow_back, color: colors.surface),
                      iconSize: AppTheme.iconSM * AppTheme.iconScale(context),
                      onPressed: () => _handleChange(widget.defaultValue!),
                    ),
                ],
              )
            : null,
      ),
      enabled: widget.enabled,
      onChanged: widget.enabled ? (v) => _handleChange(v) : null,
    );
  }

  Widget _buildDropdown(String value, TextTheme textTheme, ColorScheme colors) {
    final isDisabled = (String? v) =>
        v != null && widget.disabledValues.contains(v);
    return DropdownButtonFormField<String>(
      value: widget.disabledValues.contains(value) ? null : value,
      style: textTheme.bodyMedium,
      decoration: InputDecoration(
        contentPadding: EdgeInsets.symmetric(
          horizontal: AppTheme.spaceSM * AppTheme.spaceScale(context),
          vertical: AppTheme.spaceSM * AppTheme.spaceScale(context),
        ),
        border: const OutlineInputBorder(
          borderRadius: BorderRadius.all(Radius.circular(AppTheme.radiusSM)),
        ),
        suffixIcon: widget.enabled && widget.defaultValue != null
            ? Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  IconButton(
                    icon: Icon(Icons.arrow_back, color: colors.surface),
                    iconSize: AppTheme.iconSM * AppTheme.iconScale(context),
                    onPressed: () => _handleChange(widget.defaultValue!),
                  ),
                ],
              )
            : null,
      ),
      items: widget.items.map((item) {
        final disabled = isDisabled(item.value);
        return DropdownMenuItem<String>(
          value: item.value,
          enabled: !disabled,
          child: Opacity(
            opacity: disabled ? 0.4 : 1.0,
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                item.child ?? const SizedBox.shrink(),
                if (disabled)
                  Padding(
                    padding: EdgeInsets.only(
                      left: AppTheme.spaceXS * AppTheme.spaceScale(context),
                    ),
                    child: Text(
                      '(unavailable)',
                      style: textTheme.labelSmall?.copyWith(
                        color: colors.onSurfaceVariant,
                      ),
                    ),
                  ),
              ],
            ),
          ),
        );
      }).toList(),
      onChanged: widget.enabled
          ? (v) {
              if (v != null && !isDisabled(v)) _handleChange(v);
            }
          : null,
    );
  }
}
