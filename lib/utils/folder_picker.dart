import 'package:flutter/material.dart';

import 'package:tawai/ui/widgets/dialog/server_folder_picker.dart';
import 'package:tawai/utils/io_service.dart';
import 'package:tawai/utils/platform_service.dart';

/// Uniform folder picker that resolves the correct backend automatically.
///
/// In remote/web mode the selected path must live on the **server**
/// filesystem, so it opens a server-side folder browser backed by the
/// `GET /api/tawai/system/fs/list` endpoint. Locally it uses the native
/// `file_picker` dialog as before.
Future<String?> pickFolder(BuildContext context, {String? initialPath}) {
  if (PlatformService().isRemote) {
    return showServerFolderPicker(context, startPath: initialPath);
  }
  return IOServiceFactory.create().getDirectoryPath();
}
