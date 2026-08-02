// Routing probe test — the medium fixture's coverage-routing witness.
//
// This test imports NO library code (it only touches `dart:io`), so it covers
// zero lib lines. Under per-test coverage routing, the runner must execute
// this test exactly ONCE (during baseline coverage collection) and never
// re-run it for any mutant. If the tool lacks routing and re-runs the full
// suite per mutant, this test executes once per mutant, appending a marker
// line each time — the routing test in crates/dart-mutant/tests/routing.rs
// counts those lines to prove routing behaviourally.
//
// The marker path comes from the DM_ROUTING_MARKER env var (set by the Rust
// integration test, inherited through the tool's child processes). When the
// env var is absent (plain `dart test` runs), the test sleeps briefly and
// stays green — it is a legitimate slow test either way.
import 'dart:io';

import 'package:test/test.dart';

void main() {
  test('routing probe: no lib coverage, writes marker when invoked', () async {
    final marker = Platform.environment['DM_ROUTING_MARKER'];
    if (marker != null && marker.isNotEmpty) {
      final f = File(marker);
      await f.writeAsString('ran\n', mode: FileMode.append, flush: true);
    }
    // A modest sleep keeps this test 'slow' relative to the others (~1s),
    // so a routing regression is visible in wall-clock too, but the marker
    // count is the real assertion — timing alone is machine-dependent.
    await Future<void>.delayed(const Duration(milliseconds: 700));
    expect(true, isTrue);
  });
}
