import 'package:tawai/src/bindings/bindings.dart';

enum SearchMode { standard, enhanced }

class SearchResultItem {
  final String filename;
  final int size;
  final String sourceType;
  final String? username;
  final String? title;
  final String? thumbnail;
  final double? duration;
  final String? channel;
  final int? bitrate;
  final String? extension;
  final String? webpageUrl;

  const SearchResultItem({
    required this.filename,
    required this.size,
    required this.sourceType,
    this.username,
    this.title,
    this.thumbnail,
    this.duration,
    this.channel,
    this.bitrate,
    this.extension,
    this.webpageUrl,
  });

  factory SearchResultItem.fromDlSearchItem(DlSearchItem item) {
    return SearchResultItem(
      filename: item.filename,
      size: item.size.toInt(),
      sourceType: item.sourceType,
      username: item.username,
      title: item.title,
      thumbnail: item.thumbnail,
      duration: item.duration,
      channel: item.channel,
      bitrate: item.bitrate,
      extension: item.extension,
      webpageUrl: item.webpageUrl,
    );
  }
}
