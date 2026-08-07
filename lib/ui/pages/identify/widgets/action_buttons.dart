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
    final busy = fingerprinting || lookingUp;
    return Padding(
      padding: EdgeInsets.all(AppTheme.spaceMD * AppTheme.spaceScale(context)),
      child: Row(
        children: [
          Expanded(
            child: FilledButton.icon(
              onPressed: busy ? null : onFingerprint,
              icon: fingerprinting
                  ? SizedBox(
                      width: AppTheme.spaceLG * AppTheme.spaceScale(context),
                      height: AppTheme.spaceLG * AppTheme.spaceScale(context),
                      child: const CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.fingerprint),
              label: const Text('Fingerprint'),
            ),
          ),
          const SizedBox(width: AppTheme.spaceSM),
          Expanded(
            child: OutlinedButton.icon(
              onPressed: busy ? null : onLookup,
              icon: lookingUp
                  ? SizedBox(
                      width: AppTheme.spaceLG * AppTheme.spaceScale(context),
                      height: AppTheme.spaceLG * AppTheme.spaceScale(context),
                      child: const CircularProgressIndicator(strokeWidth: 2),
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
