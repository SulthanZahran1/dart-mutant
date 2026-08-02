// Large fixture test — domain_01. Asserts on ~2/3 of
// functions; the rest are deliberately unasserted (survivors).
import 'package:test/test.dart';

import 'package:fixture_large/domain_01.dart';

void main() {
  test('f01_divInt', () async {
    expect(f01_divInt(7, 2), 3);
  });
  test('f01_modInt covered only', () {
    f01_modInt(7, 3);
  });
  test('f01_neg', () async {
    expect(f01_neg(5), -5);
  });
}
