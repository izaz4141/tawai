import 'package:flutter/foundation.dart';

import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/settings.dart';

/// Shared ListenBrainz token validation.
///
/// The token belongs to the Connections tab while the recommendation sources
/// it unlocks belong to the Library tab, so the validation result lives here
/// rather than inside either tab.
class ListenBrainzState {
  static final ListenBrainzState instance = ListenBrainzState._();
  ListenBrainzState._();

  /// True when the token currently saved in [SettingsManager] is valid.
  final ValueNotifier<bool> savedTokenValid = ValueNotifier<bool>(false);

  String? _validatedToken;

  /// Validates the saved token unless the same token has already passed.
  Future<void> validateSavedToken({bool force = false}) async {
    final token = SettingsManager.listenbrainzToken.value.trim();
    if (token.isEmpty) {
      savedTokenValid.value = false;
      _validatedToken = null;
      return;
    }
    if (!force && _validatedToken == token && savedTokenValid.value) return;
    final result = await BridgeService.instance.validateLBToken(token);
    // Drop stale results: the saved token changed while this request ran.
    if (SettingsManager.listenbrainzToken.value.trim() != token) return;
    savedTokenValid.value = result.valid;
    _validatedToken = result.valid ? token : null;
  }

  /// Records the outcome of an explicit test against [token].
  void markValidated(String token, bool valid) {
    _validatedToken = valid ? token : null;
    savedTokenValid.value = valid;
  }
}
