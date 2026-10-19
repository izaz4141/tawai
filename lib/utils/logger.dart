import 'package:intl/intl.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/log_service.dart';

// [log] takes a LogLevel, so importing the logger has to provide it.
export 'package:tawai/utils/log_service.dart' show LogLevel;

void log(String message, {LogLevel level = LogLevel.debug}) {
  final timestamp = DateFormat('yy/MM/dd|HH:mm:ss').format(DateTime.now());
  final logMessage = '[${logLevelLabel(level)}][$timestamp] $message';
  // ignore: avoid_print
  print(logMessage);
}

void initRustSignalLogger() {
  BridgeService.instance.logSignal.listen((signal) {
    log(signal.message, level: logLevelFromString(signal.level));
  });
}
