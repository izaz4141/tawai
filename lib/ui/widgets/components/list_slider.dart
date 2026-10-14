import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import 'package:tawai/ui/theme/app_theme.dart';

// A special "sentinel" slider end with its own label (e.g. a "Lossless" end
// parked one step past the numeric range). Its `value` supersedes `min`/`max`.
typedef ListSliderEnd<T extends num> = ({String label, T value});

class ListSlider<T extends num> extends StatelessWidget {
  final String title;
  final String subtitle;
  final ValueListenable<T> valueListenable;

  /// Fallback numeric lower bound, used as the slider minimum only when
  /// [minValue] is not supplied.
  final T min;

  /// Fallback numeric upper bound, used as the slider maximum only when
  /// [maxValue] is not supplied.
  final T max;

  /// Optional special lower end.
  ///
  /// When set it supersedes [min] as the slider minimum and its label is shown
  /// at the left end. Intended for a value that is conceptually distinct from
  /// the numeric range rather than a regular step (e.g. "Original"), so its
  /// [ListSliderEnd.value] may sit outside `[min, max]`. It should still be
  /// reachable by the step grid.
  final ListSliderEnd<T>? minValue;

  /// Optional special upper end. See [minValue].
  ///
  /// When set it supersedes [max] as the slider maximum and its label is shown
  /// at the right end — e.g. "Lossless" parked one step above the highest
  /// numeric value.
  final ListSliderEnd<T>? maxValue;

  final T? step;
  final String unit;
  final String Function(T)? formatValue;
  final T? defaultValue;

  /// Called on every change while dragging (live preview). Pair with
  /// [onChangeEnd] to persist the final value once per gesture.
  final void Function(T)? onChanged;

  /// Called once when a drag ends (or when a discrete change such as the reset
  /// button is applied). Use this for side effects like persisting the value.
  final void Function(T)? onChangeEnd;

  final bool enabled;

  const ListSlider({
    super.key,
    required this.title,
    this.subtitle = '',
    required this.valueListenable,
    required this.min,
    required this.max,
    this.minValue,
    this.maxValue,
    this.step,
    this.unit = '',
    this.formatValue,
    this.defaultValue,
    this.onChanged,
    this.onChangeEnd,
    this.enabled = true,
  });

  T get _stepValue => (step ?? (T == int ? 1 : 1.0)) as T;

  T get _low => minValue?.value ?? min;

  T get _high => maxValue?.value ?? max;

  int? get _divisions {
    final span = _high - _low;
    if (span <= 0) return null;
    final divisions = (span / _stepValue).round();
    return divisions > 0 ? divisions : null;
  }

  String _format(T value) {
    if (minValue != null && value == minValue!.value) return minValue!.label;
    if (maxValue != null && value == maxValue!.value) return maxValue!.label;
    if (formatValue != null) return formatValue!(value);
    return unit.isEmpty ? '$value' : '$value $unit';
  }

  String _endLabel(ListSliderEnd<T>? end, T fallback) {
    if (end != null) return end.label;
    return unit.isEmpty ? '$fallback' : '$fallback $unit';
  }

  /// Convert `v` to `T`, preserving the correct runtime type (`int` vs `double`).
  T _coerce(num v) => T == int ? v.round() as T : v.toDouble() as T;

  T _quantize(double raw) {
    final low = _low.toDouble();
    final high = _high.toDouble();
    final step = _stepValue.toDouble();
    var v = raw.clamp(low, high);
    if (step > 0) {
      v = (low + ((v - low) / step).round() * step).clamp(low, high);
    }
    return _coerce(v);
  }

  void _handleChange(T newValue) {
    if (!enabled) return;
    if (onChanged != null) {
      onChanged!(newValue);
    } else if (valueListenable is ValueNotifier<T>) {
      (valueListenable as ValueNotifier<T>).value = newValue;
    }
  }

  /// Apply a discrete change (e.g. the reset button) and notify [onChangeEnd]
  /// so it is committed even though no drag is in progress.
  void _commit(T newValue) {
    _handleChange(newValue);
    onChangeEnd?.call(newValue);
  }

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final colors = Theme.of(context).colorScheme;
    final iconScale = AppTheme.iconScale(context);
    final spaceScale = AppTheme.spaceScale(context);

    return ValueListenableBuilder<T>(
      valueListenable: valueListenable,
      builder: (context, value, _) {
        final current = _coerce(value.clamp(_low, _high));
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            ListTile(
              title: Text(title, style: textTheme.bodyMedium),
              subtitle: subtitle.isNotEmpty
                  ? Text(
                      subtitle,
                      style: textTheme.bodySmall?.copyWith(
                        color: colors.onSurfaceVariant,
                      ),
                    )
                  : null,
              trailing: defaultValue != null && value != defaultValue
                  ? IconButton(
                      icon: const Icon(Icons.arrow_back),
                      iconSize: AppTheme.iconSM * iconScale,
                      onPressed: enabled ? () => _commit(defaultValue!) : null,
                    )
                  : null,
            ),
            Padding(
              padding: EdgeInsets.symmetric(
                horizontal: AppTheme.spaceMD * spaceScale,
              ),
              child: Row(
                children: [
                  Text(
                    _endLabel(minValue, min),
                    style: textTheme.labelSmall?.copyWith(
                      color: colors.onSurfaceVariant,
                    ),
                  ),
                  Expanded(
                    child: Slider(
                      min: _low.toDouble(),
                      max: _high.toDouble(),
                      divisions: _divisions,
                      value: current.toDouble(),
                      label: _format(current),
                      onChanged: enabled
                          ? (v) => _handleChange(_quantize(v))
                          : null,
                      onChangeEnd: enabled && onChangeEnd != null
                          ? (v) => onChangeEnd!(_quantize(v))
                          : null,
                    ),
                  ),
                  Text(
                    _endLabel(maxValue, max),
                    style: textTheme.labelSmall?.copyWith(
                      color: colors.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            ),
          ],
        );
      },
    );
  }
}
