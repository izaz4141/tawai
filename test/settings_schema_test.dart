import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:toml/toml.dart';

void main() {
  test('settings schema manifest parses as TOML', () {
    final document = TomlDocument.parse(
      File('assets/docs/settings_schema.toml').readAsStringSync(),
    ).toMap();

    expect(document['version'], 1);

    final settings = document['settings'] as Map<String, dynamic>;
    expect(settings['server.port']['default'], 8181);
    expect(
      (settings['library.organization.naming_pattern']['default'] as String)
          .contains('{album_artist'),
      isTrue,
    );
  });
}
