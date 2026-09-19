import 'dart:async';
import 'package:intl/intl.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/log_service.dart';

void log(String message, {LogLevel level = LogLevel.debug}) {
  final timestamp = DateFormat('yy/MM/dd|HH:mm:ss').format(DateTime.now());
  final logMessage = '[${logLevelLabel(level)}][$timestamp] $message';
  print(logMessage);
}

StreamSubscription? _logSub;

void initRustSignalLogger() {
  _logSub = BridgeService.instance.logSignal.listen((signal) {
    log(signal.message, level: logLevelFromString(signal.level));
  });
}
