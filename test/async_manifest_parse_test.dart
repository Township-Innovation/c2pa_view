import 'dart:io';

import 'package:c2pa_view/c2pa_view.dart';
import 'package:c2pa_view/core/bridge/rust_lib_init.dart';
import 'package:flutter_test/flutter_test.dart';

/// Relative to the c2pa_view package root → monorepo `c2pa/evidence/`.
const _signedJpegFixture =
    '../../../c2pa/evidence/generator-ios/samples/photo_1779180248379.jpg';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('async manifest parse', () {
    test('fromLocalPath round-trip on signed JPEG fixture', () async {
      await initRustLib();

      final fixture = File(_signedJpegFixture);
      if (!fixture.existsSync()) {
        // Evidence corpus may be absent in minimal checkouts.
        return;
      }

      final store = await ManifestStore.fromLocalPath(fixture.path);

      expect(store, isNotNull);
      expect(store!.activeManifest, isNotNull);
      expect(store.manifests, isNotEmpty);
    });
  });
}
