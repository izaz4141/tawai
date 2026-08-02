class User {
  final String id;
  final String username;
  final String displayName;
  final String passwordHash;
  final String apiKey;
  final String role;
  final String createdAt;
  final String updatedAt;

  User({
    required this.id,
    required this.username,
    this.displayName = '',
    this.passwordHash = '',
    this.apiKey = '',
    this.role = 'user',
    String? createdAt,
    String? updatedAt,
  })  : createdAt = createdAt ?? DateTime.now().toUtc().toIso8601String(),
        updatedAt = updatedAt ?? DateTime.now().toUtc().toIso8601String();

  static User fromJson(Map<String, dynamic> json) {
    return User(
      id: json['id'] ?? '',
      username: json['username'] ?? '',
      displayName: json['display_name'] ?? '',
      passwordHash: json['password_hash'] ?? '',
      apiKey: json['api_key'] ?? '',
      role: json['role'] ?? 'user',
      createdAt: json['created_at'],
      updatedAt: json['updated_at'],
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'username': username,
      'display_name': displayName,
      'password_hash': passwordHash,
      'api_key': apiKey,
      'role': role,
      'created_at': createdAt,
      'updated_at': updatedAt,
    };
  }
}
