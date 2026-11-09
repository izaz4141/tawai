import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'package:tawai/models/user.dart';
import 'package:tawai/ui/pages/settings/modals/connect_service.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/app_snackbar.dart';
import 'package:tawai/ui/widgets/components/list_button.dart';
import 'package:tawai/ui/widgets/components/list_switch.dart';
import 'package:tawai/ui/widgets/components/list_text_field.dart';
import 'package:tawai/ui/widgets/components/section_header.dart';
import 'package:tawai/ui/widgets/components/spin_box.dart';
import 'package:tawai/ui/widgets/dialog/db_account_manager_dialog.dart';
import 'package:tawai/utils/api_service.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/listenbrainz_state.dart';
import 'package:tawai/utils/settings.dart';
import 'package:tawai/utils/system_service.dart';

class SettingsConnectionsTab extends StatefulWidget {
  const SettingsConnectionsTab({super.key});

  @override
  State<SettingsConnectionsTab> createState() => _SettingsConnectionsTabState();
}

class _SettingsConnectionsTabState extends State<SettingsConnectionsTab> {
  final _obscured = ValueNotifier<bool>(true);

  bool _lbTokenValid = false;
  bool _lbTokenTesting = false;
  String? _lbTestedToken;
  String? _lbTestMessage;
  String? _lbTestedUserName;
  late final TextEditingController _lbTokenController;

  String get _lbTypedToken => _lbTokenController.text.trim();
  String get _lbSavedToken => SettingsManager.listenbrainzToken.value.trim();

  /// The last test result applies to what is currently in the field.
  bool get _lbTestApplies {
    final tested = _lbTestedToken;
    if (tested == null) return false;
    final typed = _lbTypedToken;
    return typed.isEmpty ? tested == _lbSavedToken : typed == tested;
  }

  /// Saving is only possible for the exact token that just passed the test.
  bool get _lbCanSave =>
      !_lbTokenTesting &&
      _lbTokenValid &&
      _lbTestedToken != null &&
      _lbTypedToken.isNotEmpty &&
      _lbTypedToken == _lbTestedToken &&
      _lbTypedToken != _lbSavedToken;

  @override
  void initState() {
    super.initState();
    SettingsManager.getCurrentUserApiKey();
    _lbTokenController = TextEditingController();
    _lbTokenController.addListener(_onLbTokenChanged);
    if (_lbSavedToken.isNotEmpty) {
      _lbTokenValid = ListenBrainzState.instance.savedTokenValid.value;
      unawaited(ListenBrainzState.instance.validateSavedToken());
    }
    unawaited(SystemService().checkServiceAvailability());
  }

  @override
  void dispose() {
    _lbTokenController.removeListener(_onLbTokenChanged);
    _lbTokenController.dispose();
    _obscured.dispose();
    super.dispose();
  }

  void _onLbTokenChanged() {
    if (mounted) setState(() {});
  }

  Future<void> _validateLbToken({bool showFeedback = true}) async {
    final typed = _lbTypedToken;
    final token = typed.isNotEmpty ? typed : _lbSavedToken;
    if (token.isEmpty) {
      setState(() {
        _lbTokenValid = false;
        _lbTestedToken = null;
        _lbTestMessage = null;
        _lbTestedUserName = null;
      });
      if (showFeedback) {
        AppSnackBar.show(
          context,
          'Enter a ListenBrainz token first',
          type: SnackType.error,
        );
      }
      return;
    }
    final testingSavedToken = token == _lbSavedToken;
    setState(() => _lbTokenTesting = true);
    final result = await BridgeService.instance.validateLBToken(token);
    if (!mounted) return;
    setState(() {
      _lbTokenTesting = false;
      _lbTokenValid = result.valid;
      _lbTestedToken = token;
      _lbTestMessage = result.message;
      _lbTestedUserName = result.userName;
    });
    if (testingSavedToken) {
      ListenBrainzState.instance.markValidated(token, result.valid);
    }
    if (showFeedback) {
      if (result.valid) {
        final user = result.userName != null && result.userName!.isNotEmpty
            ? ' — ${result.userName}'
            : '';
        AppSnackBar.show(
          context,
          'ListenBrainz token is valid$user',
          type: SnackType.success,
        );
      } else {
        AppSnackBar.show(
          context,
          'Invalid ListenBrainz token: ${result.message}',
          type: SnackType.error,
        );
      }
    }
  }

  String _lbTestStatusLabel() {
    if (_lbTokenValid) {
      final user = _lbTestedUserName;
      return user != null && user.isNotEmpty
          ? 'Token validated — $user'
          : 'Token validated';
    }
    final message = _lbTestMessage;
    return message != null && message.isNotEmpty
        ? 'Token test failed: $message'
        : 'Token test failed';
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final isAdmin = SettingsManager.currentUser.value?.role == 'admin';

    return ListView(
      padding: EdgeInsets.symmetric(
        horizontal: AppTheme.spaceLG * AppTheme.spaceScale(context),
        vertical: AppTheme.spaceLG,
      ),
      children: [
        const SectionHeader(
          title: 'Account',
          leading: Icon(Icons.person_outline),
        ),
        SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
        _ApiKeyTile(obscured: _obscured),
        if (isAdmin) ...[
          SizedBox(height: AppTheme.spaceXL * AppTheme.spaceScale(context)),
          SectionHeader(
            title: 'Server',
            leading: const Icon(Icons.dns),
            trailing: ValueListenableBuilder<bool>(
              valueListenable: APIService.isOnline,
              builder: (context, online, _) {
                return IconButton(
                  tooltip: 'Restart server',
                  onPressed: () => SettingsManager.restartServer(),
                  icon: Icon(
                    Icons.restart_alt,
                    color: online ? colors.primary : colors.error,
                  ),
                );
              },
            ),
          ),
          SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
          // On web the origin decides where the server lives and login is
          // always required, so these controls cannot mean anything there.
          if (!kIsWeb) ...[
            ListTextField(
              title: 'Server Host',
              subtitle: 'Hostname or IP address of the server',
              valueListenable: SettingsManager.serverHost,
              defaultValue:
                  SettingsManager.defaultFor('server.host') as String?,
            ),
            SpinBox<int>(
              title: 'Server Port',
              subtitle: 'Port the server listens on',
              valueListenable: SettingsManager.serverPort,
              min: 1,
              max: 65535,
              defaultValue: SettingsManager.defaultFor('server.port') as int?,
            ),
            SizedBox(height: AppTheme.spaceMD * AppTheme.spaceScale(context)),
            ListSwitch(
              title: 'Require Login',
              subtitle: 'Enforce authentication for remote access',
              valueListenable: SettingsManager.requireLogin,
              defaultValue:
                  SettingsManager.defaultFor('server.require_login') as bool?,
            ),
            SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
          ],
          OutlinedButton.icon(
            onPressed: () {
              showDialog(
                context: context,
                builder: (ctx) => const DbAccountManagerDialog(),
              );
            },
            icon: Icon(
              Icons.manage_accounts,
              size: AppTheme.iconMD * AppTheme.iconScale(context),
            ),
            label: const Text('Manage Accounts'),
          ),
          SizedBox(height: AppTheme.spaceXL * AppTheme.spaceScale(context)),
          const SectionHeader(
            title: 'Download Services',
            leading: Icon(Icons.download),
          ),
          SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
          ListButton(
            title: 'slskd',
            subtitle: 'Configure slskd connection',
            onPressed: () => showConnectServiceDialog(
              context: context,
              inputs: [
                (
                  'slskd URL',
                  'Your slskd server URL',
                  false,
                  SettingsManager.slskdUrl.value,
                ),
                (
                  'slskd Token',
                  'Stored encrypted server-side; blank keeps the existing token',
                  true,
                  '',
                ),
              ],
              test: (values) async {
                final res = await BridgeService.instance.testConnection(
                  'slskd',
                  url: values[0],
                  token: values[1],
                );
                return res.success
                    ? 'Connected! v${res.version ?? '?'}'
                    : res.error ?? 'Connection failed';
              },
              onConfirm: (values) async {
                SettingsManager.slskdUrl.value = values[0];
                await SettingsManager.saveSlskdApiKey(values[1]);
                SystemService().checkServiceAvailability();
              },
            ),
            type: ListButtonType.text,
            label: 'Configure',
          ),
          SizedBox(height: AppTheme.spaceMD * AppTheme.spaceScale(context)),
          ListButton(
            title: 'Nadekodon (YouTube)',
            subtitle: 'Configure nadekodon connection',
            onPressed: () => showConnectServiceDialog(
              context: context,
              inputs: [
                (
                  'Nadekodon URL',
                  'Your nadekodon server URL',
                  false,
                  SettingsManager.nadekodonUrl.value,
                ),
                (
                  'Nadekodon Token',
                  'Stored encrypted server-side; blank keeps the existing token',
                  true,
                  '',
                ),
              ],
              test: (values) async {
                final res = await BridgeService.instance.testConnection(
                  'nadekodon',
                  url: values[0],
                  token: values[1],
                );
                return res.success
                    ? 'Connected! v${res.version ?? '?'}'
                    : res.error ?? 'Connection failed';
              },
              onConfirm: (values) async {
                SettingsManager.nadekodonUrl.value = values[0];
                await SettingsManager.saveNadekodonApiKey(values[1]);
                SystemService().checkServiceAvailability();
              },
            ),
            type: ListButtonType.text,
            label: 'Configure',
          ),
        ],
        SizedBox(height: AppTheme.spaceXL * AppTheme.spaceScale(context)),
        const SectionHeader(
          title: 'ListenBrainz',
          leading: Icon(Icons.explore),
        ),
        SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
        ListTextField(
          title: 'ListenBrainz Token',
          subtitle: 'Required for scrobbling and music discovery',
          valueListenable: SettingsManager.listenbrainzToken,
          controller: _lbTokenController,
          isObscured: true,
          confirmEnabled: _lbCanSave,
          onConfirm: (value) {
            if (!_lbCanSave) return;
            final token = value.trim();
            SettingsManager.saveUserSetting(
              SettingsManager.listenbrainzToken,
              'listenbrainz_token',
              token,
            );
            ListenBrainzState.instance.markValidated(token, _lbTokenValid);
            setState(() {});
          },
          suffixWidgets: [
            IconButton(
              icon: _lbTokenTesting
                  ? SizedBox(
                      width:
                          AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
                      height:
                          AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
                      child: const CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.bolt),
              iconSize: AppTheme.iconSM * AppTheme.iconScale(context),
              tooltip: 'Test token',
              color: _lbTestApplies && _lbTokenValid ? colors.primary : null,
              onPressed: _lbTokenTesting ? null : _validateLbToken,
            ),
          ],
        ),
        if (_lbTestApplies && _lbTestMessage != null) ...[
          SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
          Row(
            children: [
              Icon(
                _lbTokenValid
                    ? Icons.check_circle_outline
                    : Icons.error_outline,
                size: AppTheme.iconSM * AppTheme.iconScale(context),
                color: _lbTokenValid ? colors.primary : colors.error,
              ),
              SizedBox(width: AppTheme.spaceXS * AppTheme.spaceScale(context)),
              Expanded(
                child: Text(
                  _lbTestStatusLabel(),
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: _lbTokenValid ? colors.primary : colors.error,
                  ),
                ),
              ),
            ],
          ),
        ],
        SizedBox(height: AppTheme.spaceLG * AppTheme.spaceScale(context)),
        Text(
          'Link your ListenBrainz account to enable scrobbling '
          'and discover new music based on your listening habits.',
          style: Theme.of(
            context,
          ).textTheme.bodySmall?.copyWith(color: colors.onSurfaceVariant),
        ),
      ],
    );
  }
}

class _ApiKeyTile extends StatelessWidget {
  const _ApiKeyTile({required this.obscured});

  final ValueNotifier<bool> obscured;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return ValueListenableBuilder<User?>(
      valueListenable: SettingsManager.currentUser,
      builder: (context, user, _) {
        final apiKey = user?.apiKey ?? '';
        final userId = user?.id ?? '';
        return ValueListenableBuilder<bool>(
          valueListenable: obscured,
          builder: (context, hidden, _) {
            return ListTile(
              title: Text('API Key', style: textTheme.bodyMedium),
              subtitle: Text(
                user == null
                    ? 'No account signed in'
                    : apiKey.isEmpty
                    ? 'No API key available'
                    : hidden
                    ? List.filled(apiKey.length, '•').join()
                    : apiKey,
                style: textTheme.bodySmall?.copyWith(
                  color: colors.onSurfaceVariant,
                  fontFamily: 'monospace',
                ),
              ),
              trailing: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  if (apiKey.isNotEmpty)
                    IconButton(
                      tooltip: hidden ? 'Show API key' : 'Hide API key',
                      icon: Icon(
                        hidden ? Icons.visibility : Icons.visibility_off,
                      ),
                      iconSize: AppTheme.iconSM * AppTheme.iconScale(context),
                      onPressed: () => obscured.value = !hidden,
                    ),
                  IconButton(
                    tooltip: 'Copy API key',
                    icon: const Icon(Icons.copy),
                    iconSize: AppTheme.iconSM * AppTheme.iconScale(context),
                    onPressed: apiKey.isEmpty
                        ? null
                        : () {
                            Clipboard.setData(ClipboardData(text: apiKey));
                            AppSnackBar.show(
                              context,
                              'API key copied',
                              type: SnackType.success,
                            );
                          },
                  ),
                  IconButton(
                    tooltip: 'Reroll API key',
                    icon: const Icon(Icons.refresh),
                    iconSize: AppTheme.iconSM * AppTheme.iconScale(context),
                    onPressed: userId.isEmpty
                        ? null
                        : () async {
                            final confirmed = await showDialog<bool>(
                              context: context,
                              builder: (ctx) => AlertDialog(
                                title: const Text('Reroll API key?'),
                                content: const Text(
                                  'Regenerating replaces your current API key. '
                                  'Clients using the old key will need to update.',
                                ),
                                actions: [
                                  TextButton(
                                    onPressed: () => Navigator.pop(ctx, false),
                                    child: const Text('Cancel'),
                                  ),
                                  FilledButton(
                                    onPressed: () => Navigator.pop(ctx, true),
                                    child: const Text('Reroll'),
                                  ),
                                ],
                              ),
                            );
                            if (confirmed != true || !context.mounted) return;
                            final newKey =
                                await SettingsManager.regenerateApiKey(userId);
                            if (!context.mounted) return;
                            if (newKey != null && newKey.isNotEmpty) {
                              obscured.value = true;
                              AppSnackBar.show(
                                context,
                                'API key rerolled',
                                type: SnackType.success,
                              );
                            } else {
                              AppSnackBar.show(
                                context,
                                'Failed to reroll API key',
                                type: SnackType.error,
                              );
                            }
                          },
                  ),
                ],
              ),
            );
          },
        );
      },
    );
  }
}
