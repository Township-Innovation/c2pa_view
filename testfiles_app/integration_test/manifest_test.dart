import 'dart:convert';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:c2pa_view/c2pa_view.dart';
import 'test_helpers.dart';

void runManifestTest() {
  test('Manifest', () async {
    // Load image from url into bytes
    final response = await http.get(Uri.parse(url));

    // Check that the response is successful
    expect(
      response.statusCode,
      200,
      reason:
          "Failed to get c2pa test image: ${response.statusCode}"
          "\nLooked for: $url"
          "\nCheck here for info: $testDataUrl",
    );

    // Get true manifest
    final manifestResponse = await http.get(Uri.parse(manifestUrl));

    // Check that the response is successful
    expect(
      manifestResponse.statusCode,
      200,
      reason:
          "Failed to get c2pa test manifest: ${manifestResponse.statusCode}"
          "\nLooked for: $manifestUrl"
          "\nCheck here for info: $testDataUrl",
    );
    Map<String, dynamic> trueManifest = json.decode(manifestResponse.body);

    // Get manifest from file
    Map<String, dynamic> manifest = json.decode(
      (await getManifestJsonFromBytes(
        fileBytes: response.bodyBytes,
        format: 'image/jpeg',
      ))!,
    );

    // Check active manifest
    expect(manifest['active_manifest'], trueManifest['active_manifest']);

    // Get set of manifests for both
    final Set<String> activeManifests = manifest['manifests'].keys.toSet();
    final Set<String> trueActiveManifests =
        trueManifest['manifests'].keys.toSet();

    // Check that the active manifests are the same
    expect(
      activeManifests,
      trueActiveManifests,
      reason: "Active manifests do not match expected active manifests",
    );
  });
}
