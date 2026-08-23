import 'dart:convert';
import 'dart:typed_data';

import 'package:c2pa_view/src/rust/api/c2pa.dart';
import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;

String? _manifestJsonFromUtf8Bytes(final Uint8List? bytes) =>
    bytes == null ? null : utf8.decode(bytes);

/// Get the manifest from a file.
///
/// When [trustAnchorsPem] is provided, validation runs against that trust
/// list (concatenated C2PA CA + TSA PEM bundle); otherwise the underlying
/// `c2pa-rs` defaults are used and certificates report as `untrusted`.
// ignore: type_annotate_public_apis
Future<String?> getManifestJsonFromFile(
  final String path, {
  final String? trustAnchorsPem,
}) async {
  if (trustAnchorsPem != null && trustAnchorsPem.isNotEmpty) {
    return getManifestWithTrustValidationFromPath(
      path: path,
      trustAnchorsPem: trustAnchorsPem,
    );
  }
  return getManifestWithValidationFromPath(path: path);
}

/// Get the manifest from a URL.
///
/// Optionally specify the format (mime type) if not in the header.
/// See [getManifestJsonFromFile] for the meaning of [trustAnchorsPem].
Future<String?> getManifestJsonFromURL(
  final String url, {
  final String? format,
  final String? trustAnchorsPem,
}) async {
  final response = await http.get(Uri.parse(url));
  final mime = response.headers['content-type'] ?? format ?? 'image/jpeg';
  if (trustAnchorsPem != null && trustAnchorsPem.isNotEmpty) {
    return getManifestWithTrustValidation(
      fileBytes: response.bodyBytes,
      format: mime,
      trustAnchorsPem: trustAnchorsPem,
    );
  }
  return getManifestJsonFromBytes(
    fileBytes: response.bodyBytes,
    format: mime,
  );
}

/// Get the manifest from bytes and format (mime type).
///
/// See [getManifestJsonFromFile] for the meaning of [trustAnchorsPem].
Future<String?> getManifestJsonFromBytes({
  required final List<int> fileBytes,
  required final String format,
  final String? trustAnchorsPem,
}) async {
  if (trustAnchorsPem != null && trustAnchorsPem.isNotEmpty) {
    return getManifestWithTrustValidation(
      fileBytes: fileBytes,
      format: format,
      trustAnchorsPem: trustAnchorsPem,
    );
  }
  if (kDebugMode) {
    debugPrint(
      'provenance: frb utf8 in len=${fileBytes.length} format=$format',
    );
  }
  try {
    final raw = await getManifestWithValidationUtf8(
      fileBytes: fileBytes,
      format: format,
    );
    if (kDebugMode) {
      debugPrint('provenance: frb utf8 out len=${raw?.length ?? 'null'}');
    }
    return _manifestJsonFromUtf8Bytes(raw);
  } on Object catch (error, stackTrace) {
    if (kDebugMode) {
      debugPrint(
        'provenance: frb utf8 failed ${error.runtimeType}: '
        '${error.toString().length > 120 ? '${error.toString().substring(0, 120)}…' : error}',
      );
      debugPrint('provenance: frb utf8 stack: $stackTrace');
    }
    rethrow;
  }
}

/// Reads a detached manifest **store** (e.g. L1 `GET /manifests/{hash}` CBOR).
///
/// Unlike [getManifestJsonFromBytes], does not treat [fileBytes] as the asset
/// under validation — detached L1 manifest-store bytes skip data_hash binding.
Future<String?> getManifestStoreJsonFromBytes({
  required final List<int> fileBytes,
  required final String format,
}) async {
  return _manifestJsonFromUtf8Bytes(
    await getFileManifestFormatUtf8(fileBytes: fileBytes, format: format),
  );
}
