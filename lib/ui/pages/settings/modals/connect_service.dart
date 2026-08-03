import 'package:flutter/material.dart';
import 'package:tawai/ui/theme/app_theme.dart';

class ConnectServiceDialog extends StatefulWidget {
  /// Input tuple order: (label, helperText, obscureText, initialValue?)
  final List<(String, String, bool, String?)> inputs;
  final Future<String?> Function(List<String> values)? test;
  final Future<void> Function(List<String> values) onConfirm;

  const ConnectServiceDialog({
    super.key,
    required this.inputs,
    this.test,
    required this.onConfirm,
  });

  @override
  State<ConnectServiceDialog> createState() => _ConnectServiceDialogState();
}

class _ConnectServiceDialogState extends State<ConnectServiceDialog> {
  final List<TextEditingController> _controllers = [];
  bool _isTesting = false;
  String? _testResult;
  bool _testSuccess = false;

  @override
  void initState() {
    super.initState();
    for (var i = 0; i < widget.inputs.length; i++) {
      _controllers.add(TextEditingController(text: widget.inputs[i].$4 ?? ''));
    }
  }

  @override
  void dispose() {
    for (final controller in _controllers) {
      controller.dispose();
    }
    super.dispose();
  }

  List<String> _getInputValues() {
    return _controllers.map((controller) => controller.text).toList();
  }

  Future<void> _runTest() async {
    if (widget.test == null) return;
    setState(() {
      _isTesting = true;
      _testResult = null;
      _testSuccess = false;
    });
    try {
      final result = await widget.test!(_getInputValues());
      if (!mounted) return;
      setState(() {
        _testResult = result;
        _testSuccess = result?.startsWith('Connected') ?? false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _testResult = 'Error: $e';
        _testSuccess = false;
      });
    } finally {
      if (mounted) {
        setState(() => _isTesting = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    return AlertDialog(
      title: Row(
        children: [
          Icon(
            Icons.link,
            size: AppTheme.iconSM * AppTheme.iconScale(context),
            color: colors.primary,
          ),
          SizedBox(width: AppTheme.spaceSM * AppTheme.spaceScale(context)),
          Text('Connect Service', style: textTheme.titleMedium),
        ],
      ),
      constraints: BoxConstraints(minWidth: AppTheme.dialogWidth(context)),
      content: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            for (var i = 0; i < widget.inputs.length; i++) ...[
              if (i > 0)
                SizedBox(
                  height: AppTheme.spaceMD * AppTheme.spaceScale(context),
                ),
              TextField(
                controller: _controllers[i],
                obscureText: widget.inputs[i].$3,
                style: textTheme.bodyMedium,
                decoration: InputDecoration(
                  labelText: widget.inputs[i].$1,
                  labelStyle: textTheme.bodyMedium?.copyWith(
                    color: colors.onSurface,
                  ),
                  helperText: widget.inputs[i].$2.isNotEmpty
                      ? widget.inputs[i].$2
                      : null,
                  helperStyle: textTheme.bodySmall,
                  helperMaxLines: 2,
                  border: OutlineInputBorder(
                    borderRadius: BorderRadius.circular(
                      AppTheme.radiusMD * AppTheme.radiusScale(context),
                    ),
                  ),
                  contentPadding: EdgeInsets.symmetric(
                    horizontal: AppTheme.spaceSM * AppTheme.spaceScale(context),
                    vertical: AppTheme.spaceSM * AppTheme.spaceScale(context),
                  ),
                ),
              ),
            ],
            if (_testResult != null) ...[
              SizedBox(height: AppTheme.spaceMD * AppTheme.spaceScale(context)),
              Text(
                _testResult!,
                style: textTheme.bodySmall?.copyWith(
                  color: _testSuccess ? Colors.green : Colors.red,
                ),
              ),
            ],
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text('Cancel', style: textTheme.bodyMedium),
        ),
        if (widget.test != null)
          TextButton(
            onPressed: _isTesting ? null : _runTest,
            child: _isTesting
                ? SizedBox(
                    width: 16,
                    height: 16,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : Text('Test', style: textTheme.bodyMedium),
          ),
        FilledButton(
          onPressed: () async {
            try {
              await widget.onConfirm(_getInputValues());
              if (mounted) Navigator.of(context).pop();
            } catch (e) {
              if (!mounted) return;
              ScaffoldMessenger.of(
                context,
              ).showSnackBar(SnackBar(content: Text('Failed to save: $e')));
            }
          },
          child: Text('Confirm', style: textTheme.bodyMedium),
        ),
      ],
    );
  }
}

Future<void> showConnectServiceDialog({
  required BuildContext context,
  required List<(String, String, bool, String?)> inputs,
  required Future<void> Function(List<String> values) onConfirm,
  Future<String?> Function(List<String> values)? test,
}) async {
  await showDialog(
    context: context,
    builder: (context) =>
        ConnectServiceDialog(inputs: inputs, test: test, onConfirm: onConfirm),
  );
}
