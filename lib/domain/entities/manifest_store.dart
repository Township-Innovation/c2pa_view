import 'dart:convert';

import 'package:c2pa_view/api.dart';
import 'package:c2pa_view/domain/entities/manifest.dart';
import 'package:c2pa_view/domain/entities/validation_status.dart';
import 'package:equatable/equatable.dart';

/// Root domain entity representing a C2PA Content Credential Manifest store.
class ManifestStore extends Equatable {
  const ManifestStore({
    this.activeManifest,
    this.manifests = const {},
    this.rawManifestJsons = const {},
    this.validationStatus = const [],
  });

  /// Creates a ManifestStore from a JSON map.
  factory ManifestStore.fromJson(final Map<String, dynamic> json) {
    // Parse top-level validation status entries.
    final topLevelValidation = <ValidationStatusEntry>[];
    if (json['validation_status'] is List) {
      for (final item in json['validation_status'] as List) {
        if (item is Map<String, dynamic>) {
          topLevelValidation.add(ValidationStatusEntry.fromJson(item));
        }
      }
    }

    final activeLabel = json['active_manifest'] as String?;

    // Group validation entries by the manifest label they reference.
    // Entry URLs look like: self#jumbf=/c2pa/<manifestLabel>/c2pa.signature
    // Entries that cannot be attributed to any specific manifest fall back
    // to the active manifest.
    final perManifestValidation = <String, List<Map<String, dynamic>>>{};
    for (final entry in topLevelValidation) {
      final manifestLabel = _extractManifestLabel(entry.url);
      final key = (manifestLabel != null) ? manifestLabel : (activeLabel ?? '');
      perManifestValidation.putIfAbsent(key, () => []).add({
        'code': entry.code,
        if (entry.url != null) 'url': entry.url,
        if (entry.explanation != null) 'explanation': entry.explanation,
      });
    }

    final rawManifestJsons = <String, Map<String, dynamic>>{};
    final manifests = <String, Manifest>{};
    final manifestsJson = json['manifests'];
    if (manifestsJson is Map) {
      for (final entry in manifestsJson.entries) {
        final raw = entry.value;
        if (raw is! Map) {
          continue;
        }
        final manifestJson = Map<String, dynamic>.from(raw);
        // Inject the entries attributed to this manifest when the
        // manifest JSON does not already carry its own validation_status.
        if (manifestJson['validation_status'] == null) {
          final entries = perManifestValidation[entry.key];
          if (entries != null && entries.isNotEmpty) {
            manifestJson['validation_status'] = entries;
          }
        }
        rawManifestJsons[entry.key] = manifestJson;
        manifests[entry.key] = Manifest.fromJson(manifestJson);
      }
    }

    return ManifestStore(
      activeManifest: activeLabel,
      manifests: manifests,
      rawManifestJsons: rawManifestJsons,
      validationStatus: topLevelValidation,
    );
  }

  /// Extracts the manifest label from a C2PA JUMBF URL.
  ///
  /// Input: `self#jumbf=/c2pa/contentauth:urn:uuid:XXXX/c2pa.signature`
  /// Output: `contentauth:urn:uuid:XXXX`
  static String? _extractManifestLabel(final String? url) {
    if (url == null) {
      return null;
    }
    // Find the /c2pa/ prefix in the fragment or path.
    const prefix = '/c2pa/';
    final idx = url.indexOf(prefix);
    if (idx == -1) {
      return null;
    }
    final afterPrefix = url.substring(idx + prefix.length);
    // The manifest label ends at the next '/' (if present).
    final slashIdx = afterPrefix.indexOf('/');
    return slashIdx == -1 ? afterPrefix : afterPrefix.substring(0, slashIdx);
  }

  /// Creates a ManifestStore from a local file path.
  ///
  /// When [trustAnchorsPem] is provided, validation runs against that
  /// trust list (concatenated C2PA CA + TSA PEM bundle from
  /// `TrustListService`); otherwise the underlying `c2pa-rs` defaults
  /// are used and certificates report as `untrusted`.
  static Future<ManifestStore?> fromLocalPath(
    final String localPath, {
    final String? trustAnchorsPem,
  }) async {
    final manifestJson = await getManifestJsonFromFile(
      localPath,
      trustAnchorsPem: trustAnchorsPem,
    );
    if (manifestJson == null) {
      return null;
    }
    return ManifestStore.fromJson(json.decode(manifestJson));
  }

  /// Creates a ManifestStore from a URL.
  ///
  /// See [fromLocalPath] for the meaning of [trustAnchorsPem].
  static Future<ManifestStore?> fromUrl(
    final String url, {
    final String? format,
    final String? trustAnchorsPem,
  }) async {
    final manifestJson = await getManifestJsonFromURL(
      url,
      format: format,
      trustAnchorsPem: trustAnchorsPem,
    );
    if (manifestJson == null) {
      return null;
    }
    return ManifestStore.fromJson(json.decode(manifestJson));
  }

  /// Creates a ManifestStore from raw bytes.
  ///
  /// See [fromLocalPath] for the meaning of [trustAnchorsPem].
  static Future<ManifestStore?> fromBytes(
    final List<int> fileBytes,
    final String format, {
    final String? trustAnchorsPem,
  }) async {
    final manifestJson = await getManifestJsonFromBytes(
      fileBytes: fileBytes,
      format: format,
      trustAnchorsPem: trustAnchorsPem,
    );
    if (manifestJson == null) {
      return null;
    }
    return ManifestStore.fromJson(json.decode(manifestJson));
  }

  /// A label for the active (most recent) manifest in the store.
  final String? activeManifest;

  /// A HashMap of Manifests.
  final Map<String, Manifest> manifests;

  /// Raw JSON maps for each manifest, keyed by label.
  /// Used to allow callers to copy or inspect the unprocessed manifest data.
  final Map<String, Map<String, dynamic>> rawManifestJsons;

  /// Top-level validation status entries.
  final List<ValidationStatusEntry> validationStatus;

  @override
  List<Object?> get props => [activeManifest, manifests, validationStatus];
}
