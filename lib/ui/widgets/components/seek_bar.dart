import 'package:flutter/material.dart';

import 'package:tawai/services/playback_service.dart';
import 'package:tawai/ui/theme/app_theme.dart';

class SeekBar extends StatefulWidget {
  final bool compact;
  final double? trackHeight;
  final double? thumbRadius;
  final double? overlayRadius;

  const SeekBar({
    super.key,
    this.compact = false,
    this.trackHeight,
    this.thumbRadius,
    this.overlayRadius,
  });

  @override
  State<SeekBar> createState() => _SeekBarState();
}

class _SeekBarState extends State<SeekBar> {
  bool _dragging = false;
  double _dragValue = 0;

  @override
  Widget build(BuildContext context) {
    final ps = PlaybackService.instance;
    return ValueListenableBuilder<Duration>(
      valueListenable: ps.duration,
      builder: (context, dur, _) {
        return ValueListenableBuilder<Duration>(
          valueListenable: ps.position,
          builder: (context, pos, _) {
            return ValueListenableBuilder<Duration>(
              valueListenable: ps.buffered,
              builder: (context, buf, _) {
                final max = dur.inMilliseconds > 0 ? dur.inMilliseconds : 1.0;
                final value = _dragging
                    ? _dragValue
                    : (pos.inMilliseconds / max).clamp(0.0, 1.0);
                final bufferedFrac = (buf.inMilliseconds / max).clamp(0.0, 1.0);
                final thumbRadius =
                    widget.thumbRadius ?? (widget.compact ? 4 : 6);
                final colors = Theme.of(context).colorScheme;
                return Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    SliderTheme(
                      data: SliderTheme.of(context).copyWith(
                        trackHeight:
                            widget.trackHeight ?? (widget.compact ? 2 : 4),
                        secondaryActiveTrackColor: colors.secondaryContainer,
                        thumbShape: RoundSliderThumbShape(
                          enabledThumbRadius: thumbRadius,
                        ),
                        overlayShape: RoundSliderOverlayShape(
                          overlayRadius:
                              widget.overlayRadius ?? (widget.compact ? 6 : 14),
                        ),
                        padding: EdgeInsets.zero,
                      ),
                      child: Material(
                        type: MaterialType.transparency,
                        child: SizedBox(
                          height: widget.compact
                              ? AppTheme.spaceSM * 2
                              : AppTheme.spaceXL,
                          child: Slider(
                            value: value,
                            secondaryTrackValue: bufferedFrac,
                            onChanged: (v) {
                              setState(() {
                                _dragging = true;
                                _dragValue = v;
                              });
                            },
                            onChangeEnd: (v) {
                              _dragging = false;
                              ps.seek(
                                Duration(milliseconds: (v * max).toInt()),
                              );
                            },
                          ),
                        ),
                      ),
                    ),
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        Text(
                          _fmt(pos),
                          style: widget.compact
                              ? Theme.of(context).textTheme.labelSmall
                              : Theme.of(context).textTheme.labelMedium,
                        ),
                        Text(
                          _fmt(dur),
                          style: widget.compact
                              ? Theme.of(context).textTheme.labelSmall
                              : Theme.of(context).textTheme.labelMedium,
                        ),
                      ],
                    ),
                  ],
                );
              },
            );
          },
        );
      },
    );
  }

  String _fmt(Duration d) {
    final min = d.inMinutes.remainder(60);
    final sec = d.inSeconds.remainder(60);
    return '${min.toString().padLeft(2, '0')}:${sec.toString().padLeft(2, '0')}';
  }
}
