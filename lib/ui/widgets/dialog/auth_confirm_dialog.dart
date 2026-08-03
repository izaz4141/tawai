import 'package:flutter/material.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/settings.dart';
import 'package:tawai/ui/theme/app_theme.dart';

class AuthConfirmDialog extends StatefulWidget {
  const AuthConfirmDialog({super.key, this.title, this.username});

  final String? title;
  final String? username;

  @override
  State<AuthConfirmDialog> createState() => _AuthConfirmDialogState();
}

class _AuthConfirmDialogState extends State<AuthConfirmDialog> {
  final _passwordCtrl = TextEditingController();
  bool _isVerifying = false;
  String? _error;

  @override
  void dispose() {
    _passwordCtrl.dispose();
    super.dispose();
  }

  Future<void> _confirm() async {
    final password = _passwordCtrl.text;
    if (password.isEmpty) return;

    final username =
        widget.username ?? SettingsManager.currentUser.value?.username ?? '';

    setState(() {
      _isVerifying = true;
      _error = null;
    });

    final valid = await BridgeService.instance.verifyCurrentPassword(
      username,
      password,
    );

    if (!mounted) return;

    if (valid) {
      Navigator.pop(context, password);
    } else {
      setState(() {
        _isVerifying = false;
        _error = "Incorrect password";
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return AlertDialog(
      title: Text(
        widget.title ?? "Confirm Password",
        style: textTheme.titleMedium,
      ),
      content: SizedBox(
        width: AppTheme.dialogWidth(context),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              controller: _passwordCtrl,
              obscureText: true,
              autofocus: true,
              style: textTheme.bodyMedium,
              decoration: InputDecoration(
                labelText: "Password",
                labelStyle: textTheme.bodyMedium?.copyWith(
                  color: colors.onSurface,
                ),
              ),
              onSubmitted: (_) => _isVerifying ? null : _confirm(),
            ),
            if (_error != null) ...[
              const SizedBox(height: AppTheme.spaceSM),
              Text(
                _error!,
                style: TextStyle(color: colors.error, fontSize: 13),
              ),
            ],
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: _isVerifying ? null : () => Navigator.pop(context),
          child: Text("Cancel", style: textTheme.bodyMedium),
        ),
        FilledButton(
          onPressed: _isVerifying ? null : _confirm,
          child: _isVerifying
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : Text("Confirm", style: textTheme.bodyMedium),
        ),
      ],
    );
  }
}
