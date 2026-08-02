import 'package:flutter/material.dart';

class LanguageOption {
  final String code;
  final String label;
  final String nativeName;

  const LanguageOption({
    required this.code,
    required this.label,
    required this.nativeName,
  });
}

const kCommonLanguages = [
  LanguageOption(code: 'auto', label: 'Auto (detect)', nativeName: ''),
  LanguageOption(code: 'jpn', label: 'Japanese', nativeName: '日本語'),
  LanguageOption(code: 'cmn', label: 'Mandarin Chinese', nativeName: '普通话'),
  LanguageOption(code: 'yue', label: 'Cantonese', nativeName: '廣東話'),
  LanguageOption(code: 'kor', label: 'Korean', nativeName: '한국어'),
  LanguageOption(code: 'tha', label: 'Thai', nativeName: 'ไทย'),
  LanguageOption(code: 'rus', label: 'Russian', nativeName: 'Русский'),
  LanguageOption(code: 'ara', label: 'Arabic', nativeName: 'العربية'),
  LanguageOption(code: 'hin', label: 'Hindi', nativeName: 'हिन्दी'),
  LanguageOption(code: 'eng', label: 'English', nativeName: 'English'),
];

class LanguagePickerDialog extends StatelessWidget {
  final String? title;
  final List<LanguageOption> languages;
  final ValueChanged<String> onSelected;

  const LanguagePickerDialog({
    super.key,
    this.title,
    this.languages = kCommonLanguages,
    required this.onSelected,
  });

  static Future<String?> show(
    BuildContext context, {
    String? title,
    List<LanguageOption>? languages,
  }) {
    return showDialog<String>(
      context: context,
      builder: (ctx) => LanguagePickerDialog(
        title: title,
        languages: languages ?? kCommonLanguages,
        onSelected: (code) => Navigator.of(ctx).pop(code),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    return AlertDialog(
      title: Text(title ?? 'Select Language'),
      content: SizedBox(
        width: 320,
        child: ListView.separated(
          shrinkWrap: true,
          itemCount: languages.length,
          separatorBuilder: (_, __) => const Divider(height: 1),
          itemBuilder: (context, index) {
            final lang = languages[index];
            return ListTile(
              dense: true,
              title: Text(lang.label,
                  style: textTheme.bodyMedium),
              subtitle: Text(lang.nativeName,
                  style: textTheme.bodySmall
                      ?.copyWith(color: colors.onSurfaceVariant)),
              trailing: Text(lang.code,
                  style: textTheme.bodySmall?.copyWith(
                      fontFamily: 'monospace',
                      color: colors.onSurfaceVariant)),
              onTap: () => onSelected(lang.code),
            );
          },
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
      ],
    );
  }
}
