import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';
import 'package:tawai/models/account.dart';
import 'package:tawai/utils/settings.dart';
import 'package:tawai/utils/api_service.dart';
import 'package:tawai/ui/widgets/dialog/add_account_dialog.dart';
// ignore: unused_import
import 'package:tawai/ui/theme/app_theme.dart';

class AccountManagerDialog extends StatelessWidget {
  const AccountManagerDialog({super.key});

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    return AlertDialog(
      title: Text("Manage Accounts", style: textTheme.titleMedium),
      content: SizedBox(
        width: AppTheme.dialogWidth(context),
        child: ValueListenableBuilder<List<Account>>(
          valueListenable: SettingsManager.accounts,
          builder: (context, accounts, _) {
            final isAdmin = SettingsManager.currentUser.value?.role == 'admin';
            return Column(
              mainAxisSize: MainAxisSize.min,
              children: kIsWeb
                  ? [
                      ListTile(
                        leading: Icon(
                          Icons.logout,
                          size: AppTheme.iconMD * AppTheme.iconScale(context),
                        ),
                        title: Text("Logout", style: textTheme.bodyMedium),
                        subtitle: Text(
                          Uri.base.origin,
                          style: textTheme.bodySmall,
                        ),
                        onTap: () {
                          SettingsManager.isLoggedIn.value = false;
                          APIService.instance.clearAuth();
                          Navigator.pop(context);
                        },
                      ),
                    ]
                  : [
                      if (accounts.isEmpty)
                        const Padding(
                          padding: EdgeInsets.all(AppTheme.spaceMD),
                          child: Text("No saved accounts"),
                        ),
                      Flexible(
                        child: ListView.builder(
                          shrinkWrap: true,
                          itemCount: accounts.length,
                          itemBuilder: (context, index) {
                            final account = accounts[index];
                            final isCurrent =
                                account.username ==
                                    SettingsManager
                                        .currentUser
                                        .value
                                        ?.username &&
                                account.host ==
                                    SettingsManager.serverHost.value &&
                                account.port ==
                                    SettingsManager.serverPort.value;

                            return ListTile(
                              leading: Icon(
                                Icons.dns,
                                size:
                                    AppTheme.iconMD *
                                    AppTheme.iconScale(context),
                              ),
                              title: Text(account.label),
                              subtitle: Text("${account.host}:${account.port}"),
                              trailing: Row(
                                mainAxisSize: MainAxisSize.min,
                                children: [
                                  if (isCurrent)
                                    Icon(
                                      Icons.check,
                                      size:
                                          AppTheme.iconMD *
                                          AppTheme.iconScale(context),
                                      color: Colors.green,
                                    ),
                                  if (isCurrent || isAdmin)
                                    IconButton(
                                      icon: Icon(
                                        Icons.edit,
                                        size:
                                            AppTheme.iconMD *
                                            AppTheme.iconScale(context),
                                      ),
                                      onPressed: () {
                                        _showEditAccountDialog(
                                          context,
                                          account,
                                        );
                                      },
                                    ),
                                  IconButton(
                                    icon: Icon(
                                      Icons.delete,
                                      size:
                                          AppTheme.iconMD *
                                          AppTheme.iconScale(context),
                                    ),
                                    onPressed: () {
                                      SettingsManager.removeAccount(account);
                                    },
                                  ),
                                ],
                              ),
                              onTap: () {
                                SettingsManager.switchAccount(account);
                                Navigator.pop(context);
                              },
                            );
                          },
                        ),
                      ),
                      const SizedBox(height: AppTheme.spaceMD),
                      if (isAdmin)
                        ElevatedButton.icon(
                          onPressed: () {
                            Navigator.pop(context); // Close manager
                            _showAddAccountDialog(context);
                          },
                          icon: Icon(
                            Icons.add,
                            size: AppTheme.iconMD * AppTheme.iconScale(context),
                          ),
                          label: Text(
                            "Add Account",
                            style: textTheme.bodyMedium,
                          ),
                        ),
                    ],
            );
          },
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

  void _showAddAccountDialog(BuildContext context) {
    showDialog(context: context, builder: (ctx) => const AddAccountDialog());
  }

  void _showEditAccountDialog(BuildContext context, Account account) {
    showDialog(
      context: context,
      builder: (ctx) => AddAccountDialog(initialAccount: account),
    );
  }
}
