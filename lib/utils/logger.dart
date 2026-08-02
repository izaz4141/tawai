import 'dart:async';
import 'package:intl/intl.dart';
import 'package:tawai/utils/bridge_service.dart';

void log(String message, {bool isError = false}) {
  final level = isError ? 'ERROR' : 'DEBUG';
  final timestamp = DateFormat('yy/MM/dd|HH:mm:ss').format(DateTime.now());
  final logMessage = '[$level][$timestamp] $message';
  print(logMessage);
}

StreamSubscription? _logSub;

void initRustSignalLogger() {
  _logSub = BridgeService.instance.logSignal.listen((signal) {
    log(signal.message, isError: signal.level == "ERROR");
  });
}
