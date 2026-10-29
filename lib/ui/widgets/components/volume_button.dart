import 'dart:math' as math;
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:tawai/services/playback_service.dart';
import 'package:tawai/ui/theme/app_theme.dart';

const double _kVolumeScrollStep = 0.02;

/// Adjusts the volume by one 2% step per scroll-wheel notch. The signal is
/// claimed through the resolver so an ancestor scrollable does not also react.
void _handleVolumeScroll(PointerSignalEvent event) {
  if (event is! PointerScrollEvent) return;
  if (event.scrollDelta.dy == 0) return;
  GestureBinding.instance.pointerSignalResolver.register(event, (event) {
    final dy = (event as PointerScrollEvent).scrollDelta.dy;
    if (dy == 0) return;
    PlaybackService.instance.adjustVolume(
      dy < 0 ? _kVolumeScrollStep : -_kVolumeScrollStep,
    );
  });
}

class VolumeButton extends StatefulWidget {
  final double iconSize;
  final double popupWidth;
  final double popupHeight;

  const VolumeButton({
    super.key,
    this.iconSize = AppTheme.iconMD,
    this.popupWidth = 48,
    this.popupHeight = 148,
  });

  @override
  State<VolumeButton> createState() => _VolumeButtonState();
}

class _VolumeButtonState extends State<VolumeButton> {
  OverlayEntry? _entry;
  final _iconKey = GlobalKey();

  @override
  void dispose() {
    _removePopup();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final ps = PlaybackService.instance;
    return ValueListenableBuilder<double>(
      valueListenable: ps.volume,
      builder: (context, vol, _) {
        final colors = Theme.of(context).colorScheme;
        return Listener(
          onPointerSignal: _handleVolumeScroll,
          child: IconButton(
            key: _iconKey,
            icon: Icon(
              _icon(vol),
              size: widget.iconSize,
              color: vol > 0 ? colors.primary : colors.onSurfaceVariant,
            ),
            onPressed: () => _togglePopup(context),
          ),
        );
      },
    );
  }

  IconData _icon(double vol) {
    if (vol <= 0) return Icons.volume_mute_rounded;
    if (vol < 0.5) return Icons.volume_down_rounded;
    return Icons.volume_up_rounded;
  }

  void _removePopup() {
    _entry?.remove();
    _entry = null;
  }

  void _togglePopup(BuildContext context) {
    if (_entry != null) {
      _removePopup();
    } else {
      _showPopup();
    }
  }

  void _showPopup() {
    final renderBox = _iconKey.currentContext?.findRenderObject() as RenderBox?;
    if (renderBox == null || !renderBox.hasSize) return;

    final offset = renderBox.localToGlobal(Offset.zero);
    final size = renderBox.size;
    final overlay = Overlay.of(context);

    _entry = OverlayEntry(
      builder: (ctx) => _VolumePopupContent(
        iconOffset: offset,
        iconSize: size,
        width: widget.popupWidth,
        height: widget.popupHeight,
        onDismiss: _removePopup,
      ),
    );

    overlay.insert(_entry!);
  }
}

class _VolumePopupContent extends StatelessWidget {
  static const double _trackW = 4;
  static const double _trackPadTop = 24;
  static const double _trackPadBottom = 12;
  static const double _thumbR = 7;

  final Offset iconOffset;
  final Size iconSize;
  final double width;
  final double height;
  final VoidCallback onDismiss;

  const _VolumePopupContent({
    required this.iconOffset,
    required this.iconSize,
    required this.width,
    required this.height,
    required this.onDismiss,
  });

  void _onChanged(Offset local, double trackH) {
    final y = math.min(math.max(local.dy, 0.0), trackH);
    PlaybackService.instance.setVolume(1 - (y / trackH));
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final ps = PlaybackService.instance;

    final popupLeft = iconOffset.dx + iconSize.width / 2 - width / 2;
    final popupTop = iconOffset.dy - height - 8;

    return GestureDetector(
      onTap: onDismiss,
      child: Stack(
        children: [
          Positioned.fill(
            child: GestureDetector(
              onTap: onDismiss,
              child: Container(color: Colors.transparent),
            ),
          ),
          Positioned(
            left: math.min(
              math.max(popupLeft, 0),
              MediaQuery.of(context).size.width - width,
            ),
            top: math.min(
              math.max(popupTop, 0),
              MediaQuery.of(context).size.height - height,
            ),
            child: Listener(
              onPointerSignal: _handleVolumeScroll,
              child: Material(
                color: Colors.transparent,
                child: ValueListenableBuilder<double>(
                  valueListenable: ps.volume,
                  builder: (ctx, value, _) {
                    final percent = (value * 100).round();
                    final trackH = height - _trackPadTop - _trackPadBottom;
                    final trackX = (width - _trackW) / 2;
                    final thumbCenterX = trackX + _trackW / 2;
                    final thumbY =
                        _trackPadTop + (1 - value) * trackH - _thumbR;
                    final textY = thumbY - 16;

                    return Container(
                      width: width,
                      height: height,
                      decoration: BoxDecoration(
                        color: colors.surfaceContainerHigh,
                        borderRadius: BorderRadius.circular(16),
                        boxShadow: [
                          BoxShadow(
                            color: colors.shadow.withAlpha(40),
                            blurRadius: 14,
                            offset: const Offset(0, 4),
                          ),
                        ],
                      ),
                      child: ClipRRect(
                        borderRadius: BorderRadius.circular(16),
                        child: Stack(
                          children: [
                            Positioned(
                              left: trackX,
                              top: _trackPadTop,
                              bottom: _trackPadBottom,
                              child: Container(
                                width: _trackW,
                                decoration: BoxDecoration(
                                  color: colors.surfaceContainerHighest,
                                  borderRadius: BorderRadius.circular(2),
                                ),
                              ),
                            ),
                            if (value > 0)
                              Positioned(
                                left: trackX,
                                bottom: _trackPadBottom,
                                height: value * trackH,
                                child: Container(
                                  width: _trackW,
                                  decoration: BoxDecoration(
                                    color: colors.primary,
                                    borderRadius: BorderRadius.circular(2),
                                  ),
                                ),
                              ),
                            for (final tv in [0.25, 0.5, 0.75])
                              Positioned(
                                left: thumbCenterX - 2.5,
                                top: _trackPadTop + (1 - tv) * trackH - 2.5,
                                child: Container(
                                  width: 5,
                                  height: 5,
                                  decoration: BoxDecoration(
                                    shape: BoxShape.circle,
                                    color: value >= tv
                                        ? colors.primary.withAlpha(100)
                                        : colors.outline.withAlpha(50),
                                  ),
                                ),
                              ),
                            Positioned(
                              left: thumbCenterX - _thumbR,
                              top: thumbY,
                              child: Container(
                                width: _thumbR * 2,
                                height: _thumbR * 2,
                                decoration: BoxDecoration(
                                  shape: BoxShape.circle,
                                  color: colors.primary,
                                  boxShadow: [
                                    BoxShadow(
                                      color: colors.primary.withAlpha(80),
                                      blurRadius: 4,
                                    ),
                                  ],
                                ),
                              ),
                            ),
                            Positioned(
                              left: 0,
                              right: 0,
                              top: textY,
                              child: Text(
                                '$percent%',
                                textAlign: TextAlign.center,
                                style: textTheme.labelSmall?.copyWith(
                                  fontWeight: FontWeight.w700,
                                  color: colors.primary,
                                  fontSize: 11,
                                ),
                              ),
                            ),
                            Positioned(
                              left: 0,
                              right: 0,
                              top: _trackPadTop,
                              height: trackH,
                              child: GestureDetector(
                                behavior: HitTestBehavior.opaque,
                                onTapDown: (d) =>
                                    _onChanged(d.localPosition, trackH),
                                onPanUpdate: (d) =>
                                    _onChanged(d.localPosition, trackH),
                              ),
                            ),
                          ],
                        ),
                      ),
                    );
                  },
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
