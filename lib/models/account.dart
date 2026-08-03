class Account {
  final String id;
  final String host;
  final int port;
  final String username;
  final String displayName;
  final String label;
  final String role;
  final String apiKey;

  Account({
    String? id,
    required this.host,
    required this.port,
    required this.username,
    this.displayName = '',
    String? label,
    this.role = 'user',
    this.apiKey = '',
  }) : id = id ?? username,
       label = label ?? host;

  static Account fromJson(Map<String, dynamic> json) {
    return Account(
      id: json['id'] as String?,
      host: json['host'] ?? '127.0.0.1',
      port: json['port'] ?? 8181,
      username: json['username'] ?? '',
      displayName: json['display_name'] ?? '',
      label: json['label'],
      role: json['role'] ?? 'user',
      apiKey: json['api_key'] ?? '',
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'host': host,
      'port': port,
      'username': username,
      'display_name': displayName,
      'label': label,
      'role': role,
      'api_key': apiKey,
    };
  }
}
