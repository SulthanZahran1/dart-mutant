// Large fixture test — domain_07. Asserts on ~2/3 of
// functions; the rest are deliberately unasserted (survivors).
import 'package:test/test.dart';

import 'package:fixture_large/domain_07.dart';

void main() {
  test('f07_pickByFlag', () async {
    expect(f07_pickByFlag(true), 'yes'); expect(f07_pickByFlag(false), 'no');
  });
  test('f07_clampTo covered only', () {
    f07_clampTo(5, 0, 10); f07_clampTo(-1, 0, 10); f07_clampTo(99, 0, 10);
  });
  test('f07_describe', () async {
    expect(f07_describe(null), 'none'); expect(f07_describe(7), 'v=7');
  });
}
