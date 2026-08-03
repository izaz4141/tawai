import 'package:flutter/material.dart';
import 'package:tawai/models/account.dart';
import 'package:tawai/utils/api_service.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/settings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/dialog/auth_confirm_dialog.dart';
import 'package:tawai/utils/helper.dart';

class AddAccountDialog extends StatefulWidget {
  const AddAccountDialog({
    super.key,
    this.initialAccount,
    this.manageUsers = false,
  });

  final Account? initialAccount;
  final bool manageUsers;

  @override
  State<AddAccountDialog> createState() => _AddAccountDialogState();
}

class _AddAccountDialogState extends State<AddAccountDialog> {
  final _formKey = GlobalKey<FormState>();
  String _protocol = 'http://';
  final _hostCtrl = TextEditingController();
  final _portCtrl = TextEditingController(text: '8080');
  final _usernameCtrl = TextEditingController();
  final _displayNameCtrl = TextEditingController();
  final _passwordCtrl = TextEditingController();
  final _labelCtrl = TextEditingController();

  String _role = 'user';

  bool _isTesting = false;
  String? _testResult;
  bool _connectionSuccess = false;
  String _capturedApiKey = '';
  String _capturedId = '';
  String _capturedUsername = '';
  String _capturedDisplayName = '';
  String _capturedRole = 'user';

  bool get _isEdit => widget.initialAccount != null;

  bool get _isLocalHost => widget.manageUsers || isLocalHost(_hostCtrl.text);

  bool get _isAdmin => SettingsManager.currentUser.value?.role == 'admin';

  @override
  void initState() {
    super.initState();
    final acc = widget.initialAccount;
    if (acc != null) {
      if (acc.host.startsWith('https://')) {
        _protocol = 'https://';
        _hostCtrl.text = acc.host.substring(8);
      } else if (acc.host.startsWith('http://')) {
        _protocol = 'http://';
        _hostCtrl.text = acc.host.substring(7);
      } else {
        _hostCtrl.text = acc.host;
      }
      _portCtrl.text = acc.port.toString();
      _usernameCtrl.text = acc.username;
      _displayNameCtrl.text = acc.displayName;
      _labelCtrl.text = acc.label == acc.host ? '' : acc.label;
      _role = acc.role;
    }
  }

  @override
  void dispose() {
    _hostCtrl.dispose();
    _portCtrl.dispose();
    _usernameCtrl.dispose();
    _displayNameCtrl.dispose();
    _passwordCtrl.dispose();
    _labelCtrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return AlertDialog(
      title: Text(
        widget.manageUsers
            ? (_isEdit ? "Edit User" : "Add User")
            : (_isEdit ? "Edit Account" : "Add Account"),
        style: textTheme.titleMedium,
      ),
      constraints: BoxConstraints(minWidth: AppTheme.dialogWidth(context)),
      content: SingleChildScrollView(
        child: Form(
          key: _formKey,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              if (!widget.manageUsers) ...[
                TextFormField(
                  controller: _labelCtrl,
                  style: textTheme.bodyMedium,
                  decoration: InputDecoration(
                    labelText: "Label (Optional)",
                    labelStyle: textTheme.bodyMedium?.copyWith(
                      color: colors.onSurface,
                    ),
                  ),
                ),
                const SizedBox(height: AppTheme.spaceSM),
                Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    SizedBox(
                      width: 120,
                      child: DropdownButtonFormField<String>(
                        initialValue: _protocol,
                        style: textTheme.bodyMedium,
                        decoration: InputDecoration(
                          labelText: "Protocol",
                          labelStyle: textTheme.bodyMedium?.copyWith(
                            color: colors.onSurface,
                          ),
                        ),
                        items: const [
                          DropdownMenuItem(
                            value: 'http://',
                            child: Text('http://'),
                          ),
                          DropdownMenuItem(
                            value: 'https://',
                            child: Text('https://'),
                          ),
                        ],
                        onChanged: (v) {
                          setState(() {
                            _protocol = v!;
                            if (_protocol == 'https://') {
                              _portCtrl.text = '443';
                            }
                          });
                        },
                      ),
                    ),
                    const SizedBox(width: AppTheme.spaceSM),
                    Expanded(
                      flex: 2,
                      child: TextFormField(
                        controller: _hostCtrl,
                        keyboardType: TextInputType.url,
                        autocorrect: false,
                        enableSuggestions: false,
                        style: textTheme.bodyMedium,
                        onChanged: (_) {
                          setState(() {
                            _testResult = null;
                            _connectionSuccess = false;
                            _capturedApiKey = '';
                            _capturedId = '';
                            _capturedUsername = '';
                            _capturedDisplayName = '';
                            _capturedRole = 'user';
                          });
                        },
                        decoration: InputDecoration(
                          labelText: "Host (IP/Domain)",
                          hintText: "127.0.0.1",
                          labelStyle: textTheme.bodyMedium?.copyWith(
                            color: colors.onSurface,
                          ),
                        ),
                        validator: (v) =>
                            v?.isNotEmpty == true ? null : "Required",
                      ),
                    ),
                    const SizedBox(width: AppTheme.spaceSM),
                    Expanded(
                      child: TextFormField(
                        controller: _portCtrl,
                        style: textTheme.bodyMedium,
                        decoration: InputDecoration(
                          labelText: "Port",
                          labelStyle: textTheme.bodyMedium?.copyWith(
                            color: colors.onSurface,
                          ),
                        ),
                        keyboardType: TextInputType.number,
                        validator: (v) =>
                            v?.isNotEmpty == true ? null : "Required",
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: AppTheme.spaceSM),
              ],
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Expanded(
                    child: TextFormField(
                      controller: _usernameCtrl,
                      style: textTheme.bodyMedium,
                      decoration: InputDecoration(
                        labelText: "Username",
                        labelStyle: textTheme.bodyMedium?.copyWith(
                          color: colors.onSurface,
                        ),
                      ),
                      validator: (v) =>
                          v?.isNotEmpty == true ? null : "Required",
                    ),
                  ),
                  const SizedBox(width: AppTheme.spaceSM),
                  Expanded(
                    child: TextFormField(
                      controller: _displayNameCtrl,
                      style: textTheme.bodyMedium,
                      decoration: InputDecoration(
                        labelText: "Display Name",
                        labelStyle: textTheme.bodyMedium?.copyWith(
                          color: colors.onSurface,
                        ),
                      ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: AppTheme.spaceSM),
              TextFormField(
                controller: _passwordCtrl,
                keyboardType: TextInputType.visiblePassword,
                style: textTheme.bodyMedium,
                decoration: InputDecoration(
                  labelText: "Password",
                  labelStyle: textTheme.bodyMedium?.copyWith(
                    color: colors.onSurface,
                  ),
                ),
                obscureText: true,
                validator: _isEdit
                    ? null
                    : (v) => v?.isNotEmpty == true ? null : "Required",
              ),
              if (_isAdmin) ...[
                const SizedBox(height: AppTheme.spaceSM),
                DropdownButtonFormField<String>(
                  initialValue: _role,
                  style: textTheme.bodyMedium,
                  decoration: InputDecoration(
                    labelText: "Role",
                    labelStyle: textTheme.bodyMedium?.copyWith(
                      color: colors.onSurface,
                    ),
                  ),
                  items: const [
                    DropdownMenuItem(value: 'user', child: Text('user')),
                    DropdownMenuItem(value: 'admin', child: Text('admin')),
                  ],
                  onChanged: (v) {
                    setState(() {
                      _role = v ?? 'user';
                    });
                  },
                ),
              ],
              const SizedBox(height: AppTheme.spaceSM),
              if (!_isLocalHost) ...[
                if (_testResult != null)
                  Text(
                    _testResult!,
                    style: TextStyle(
                      color: _connectionSuccess ? Colors.green : Colors.red,
                    ),
                  ),
                const SizedBox(height: AppTheme.spaceSM),
                ElevatedButton(
                  onPressed: _isTesting ? null : _runTestConnection,
                  child: _isTesting
                      ? const SizedBox(
                          width: 16,
                          height: 16,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : Text(
                          "Test Connection",
                          style: textTheme.bodyMedium?.copyWith(
                            color: colors.primary,
                          ),
                        ),
                ),
              ],
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: Text("Cancel", style: textTheme.bodyMedium),
        ),
        FilledButton(
          onPressed: (!_isLocalHost && !_connectionSuccess)
              ? null
              : _saveAccount,
          child: Text("Save", style: textTheme.bodyMedium),
        ),
      ],
    );
  }

  Future<void> _runTestConnection() async {
    if (!_formKey.currentState!.validate()) return;

    setState(() {
      _isTesting = true;
      _testResult = null;
      _connectionSuccess = false;
    });

    final host = '$_protocol${_hostCtrl.text}';
    final port = _portCtrl.text;
    final username = _usernameCtrl.text;
    final password = _passwordCtrl.text;

    try {
      final result = await APIService.instance.testLogin(
        host: host,
        port: int.parse(port),
        username: username,
        password: password,
      );

      setState(() {
        _connectionSuccess = result.success;
        _testResult = result.success
            ? "Connection Successful!"
            : "Login Failed: Invalid credentials or host unreachable.";
        if (result.success) {
          _capturedApiKey = result.apiKey;
          _capturedId = result.id;
          _capturedUsername = result.username;
          _capturedDisplayName = result.displayName;
          _capturedRole = result.role;
          if (_displayNameCtrl.text.isEmpty && result.displayName.isNotEmpty) {
            _displayNameCtrl.text = result.displayName;
          }
        }
      });
    } catch (e) {
      setState(() {
        _connectionSuccess = false;
        _testResult = "Error: $e";
      });
    } finally {
      setState(() {
        _isTesting = false;
      });
    }
  }

  Future<void> _saveAccount() async {
    if (!_formKey.currentState!.validate()) return;

    String apiKey = _capturedApiKey;
    String displayName = _capturedDisplayName;
    String role = _isLocalHost ? _role : _capturedRole;
    String id = _isEdit ? widget.initialAccount!.id : _capturedId;

    if (_isEdit) {
      apiKey = apiKey.isNotEmpty ? apiKey : widget.initialAccount!.apiKey;
      displayName = displayName.isNotEmpty
          ? displayName
          : _displayNameCtrl.text.isEmpty
          ? widget.initialAccount!.displayName
          : _displayNameCtrl.text;
      role = _role;
      id = widget.initialAccount!.id;
    }

    Account account = Account(
      id: id,
      host: '$_protocol${_hostCtrl.text}',
      port: int.parse(_portCtrl.text),
      username: _usernameCtrl.text,
      displayName: displayName,
      label: _labelCtrl.text.isEmpty ? _hostCtrl.text : _labelCtrl.text,
      role: role,
      apiKey: apiKey,
    );

    if (!_isLocalHost && !_isEdit) {
      account = Account(
        id: _capturedId.isEmpty ? account.username : _capturedId,
        host: account.host,
        port: account.port,
        username: _capturedUsername.isEmpty
            ? account.username
            : _capturedUsername,
        displayName: _capturedDisplayName,
        label: account.label,
        role: _capturedRole,
        apiKey: _capturedApiKey,
      );
      SettingsManager.addAccount(account);
      if (!mounted) return;
      Navigator.pop(context);
      return;
    }

    if (_isLocalHost && !_isEdit && !widget.manageUsers) {
      final users = await BridgeService.instance.listUsers();
      if (!mounted) return;
      final index = users.indexWhere((u) => u.username == _usernameCtrl.text);
      if (index != -1) {
        final existing = users[index];
        final valid = await BridgeService.instance.verifyCurrentPassword(
          existing.username,
          _passwordCtrl.text,
        );
        if (!mounted) return;
        if (!valid) {
          _showError(context, "User exists but password is wrong");
          return;
        }
        SettingsManager.addAccount(
          Account(
            id: existing.id,
            host: account.host,
            port: account.port,
            username: existing.username,
            displayName: existing.displayName,
            label: account.label,
            role: existing.role,
          ),
        );
        Navigator.pop(context);
        return;
      }
    }

    final adminPassword = await showDialog<String>(
      context: context,
      builder: (_) => const AuthConfirmDialog(),
    );
    if (adminPassword == null || !mounted) return;

    final ({
      bool success,
      String userId,
      String username,
      String displayName,
      String role,
      String apiKey,
    })
    result;
    if (_isEdit) {
      result = await BridgeService.instance.updateAccount(
        currentUsername: SettingsManager.currentUser.value?.username ?? '',
        currentPassword: adminPassword,
        targetUsername: widget.initialAccount!.username,
        newUsername: _usernameCtrl.text,
        newPassword: _passwordCtrl.text,
        newDisplayName: _displayNameCtrl.text,
        role: _role,
      );
    } else {
      result = await BridgeService.instance.createAccount(
        adminUsername: SettingsManager.currentUser.value?.username ?? '',
        adminPassword: adminPassword,
        username: _usernameCtrl.text,
        password: _passwordCtrl.text,
        displayName: _displayNameCtrl.text,
        role: _role,
      );
    }

    if (!mounted) return;

    if (result.success) {
      account = Account(
        id: result.userId.isNotEmpty ? result.userId : account.id,
        host: account.host,
        port: account.port,
        username: result.username.isNotEmpty
            ? result.username
            : account.username,
        displayName: result.displayName.isNotEmpty
            ? result.displayName
            : account.displayName,
        label: account.label,
        role: result.role.isNotEmpty ? result.role : account.role,
        apiKey: result.apiKey.isNotEmpty ? result.apiKey : account.apiKey,
      );
      if (!widget.manageUsers) {
        SettingsManager.addAccount(account);
      }
      Navigator.pop(context);
    } else {
      _showError(
        context,
        widget.manageUsers
            ? (_isEdit ? "Failed to update user" : "Failed to create user")
            : (_isEdit
                  ? "Failed to update account"
                  : "Failed to create account"),
      );
    }
  }

  void _showError(BuildContext context, String message) {
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(message)));
  }
}
