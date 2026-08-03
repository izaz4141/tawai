import 'package:flutter/material.dart';

import 'package:tawai/models/account.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/app_snackbar.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/settings.dart';
import 'package:tawai/ui/widgets/dialog/add_account_dialog.dart';
import 'package:tawai/ui/widgets/dialog/auth_confirm_dialog.dart';

class DbAccountManagerDialog extends StatefulWidget {
  const DbAccountManagerDialog({super.key});

  @override
  State<DbAccountManagerDialog> createState() => _DbAccountManagerDialogState();
}

class _DbAccountManagerDialogState extends State<DbAccountManagerDialog> {
  List<UserListItem> _users = [];
  bool _loading = true;

  bool get _isAdmin => SettingsManager.currentUser.value?.role == 'admin';

  String get _currentUsername =>
      SettingsManager.currentUser.value?.username ?? '';

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() => _loading = true);
    final users = await BridgeService.instance.listUsers();
    if (!mounted) return;
    setState(() {
      _users = users;
      _loading = false;
    });
  }

  Future<void> _addUser() async {
    await showDialog(
      context: context,
      builder: (_) => const AddAccountDialog(manageUsers: true),
    );
    _load();
  }

  Future<void> _editUser(UserListItem user) async {
    await showDialog(
      context: context,
      builder: (_) => AddAccountDialog(
        manageUsers: true,
        initialAccount: Account(
          id: user.id,
          host: '',
          port: 8181,
          username: user.username,
          displayName: user.displayName,
          role: user.role,
        ),
      ),
    );
    _load();
  }

  Future<void> _deleteUser(UserListItem user) async {
    final adminPassword = await showDialog<String>(
      context: context,
      builder: (_) => const AuthConfirmDialog(),
    );
    if (adminPassword == null || !mounted) return;

    final result = await BridgeService.instance.deleteUser(
      adminUsername: _currentUsername,
      adminPassword: adminPassword,
      targetUsername: user.username,
    );
    if (!mounted) return;

    if (result.success) {
      _load();
    } else {
      AppSnackBar.show(
        context,
        "Failed to delete ${user.username}",
        type: SnackType.error,
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final colors = Theme.of(context).colorScheme;
    final iconScale = AppTheme.iconScale(context);
    return AlertDialog(
      title: Text("Manage Users", style: textTheme.titleMedium),
      content: SizedBox(
        width: AppTheme.dialogWidth(context),
        child: _loading
            ? const Center(child: CircularProgressIndicator())
            : Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  if (_users.isEmpty)
                    const Padding(
                      padding: EdgeInsets.all(AppTheme.spaceMD),
                      child: Text("No users found"),
                    ),
                  Flexible(
                    child: ListView.builder(
                      shrinkWrap: true,
                      itemCount: _users.length,
                      itemBuilder: (context, index) {
                        final user = _users[index];
                        final isCurrent = user.username == _currentUsername;
                        return ListTile(
                          leading: Icon(
                            Icons.person,
                            size: AppTheme.iconMD * iconScale,
                          ),
                          title: Text(
                            user.displayName.isEmpty
                                ? user.username
                                : user.displayName,
                            style: textTheme.bodyMedium,
                          ),
                          subtitle: Text(
                            user.role == 'admin'
                                ? "${user.username} · admin"
                                : user.username,
                            style: textTheme.bodySmall?.copyWith(
                              color: colors.onSurfaceVariant,
                            ),
                          ),
                          trailing: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              if (isCurrent)
                                Icon(
                                  Icons.check,
                                  size: AppTheme.iconMD * iconScale,
                                  color: Colors.green,
                                ),
                              if (_isAdmin)
                                IconButton(
                                  icon: Icon(
                                    Icons.edit,
                                    size: AppTheme.iconMD * iconScale,
                                  ),
                                  onPressed: () => _editUser(user),
                                ),
                              if (_isAdmin)
                                IconButton(
                                  icon: Icon(
                                    Icons.delete,
                                    size: AppTheme.iconMD * iconScale,
                                  ),
                                  onPressed: () => _deleteUser(user),
                                ),
                            ],
                          ),
                        );
                      },
                    ),
                  ),
                  if (_isAdmin) ...[
                    const SizedBox(height: AppTheme.spaceMD),
                    ElevatedButton.icon(
                      onPressed: _addUser,
                      icon: Icon(Icons.add, size: AppTheme.iconMD * iconScale),
                      label: Text("Add User", style: textTheme.bodyMedium),
                    ),
                  ],
                ],
              ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: Text("Close", style: textTheme.bodyMedium),
        ),
      ],
    );
  }
}
