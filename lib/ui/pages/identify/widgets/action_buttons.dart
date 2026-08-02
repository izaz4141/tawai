import 'package:flutter/material.dart';
import 'package:tawai/ui/theme/app_theme.dart';

class ActionButtons extends StatelessWidget {
  final bool fingerprinting;
  final bool lookingUp;
  final VoidCallback? onFingerprint;
  final VoidCallback? onLookup;

  const ActionButtons({
    super.key,
    this.fingerprinting = false,
    this.lookingUp = false,
    this.onFingerprint,
    this.onLookup,
  });

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return Padding(
      padding: EdgeInsets.all(AppTheme.spaceMD * AppTheme.spaceScale(context)),
      child: Row(
        children: [
          Expanded(
            child: FilledButton.icon(
              onPressed: fingerprinting ? null : onFingerprint,
              icon: fingerprinting
                  ? const SizedBox(
                      width: 18,
                      height: 18,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.fingerprint),
              label: const Text('Fingerprint'),
            ),
          ),
          const SizedBox(width: AppTheme.spaceSM),
          Expanded(
            child: OutlinedButton.icon(
              onPressed: lookingUp ? null : onLookup,
              icon: lookingUp
                  ? const SizedBox(
                      width: 18,
                      height: 18,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : Icon(Icons.search, color: colors.primary),
              label: Text('Lookup', style: TextStyle(color: colors.primary)),
            ),
          ),
        ],
      ),
    );
  }
}
