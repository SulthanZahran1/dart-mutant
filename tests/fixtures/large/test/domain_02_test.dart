// Large fixture test — domain_02. Asserts on ~2/3 of
// functions; the rest are deliberately unasserted (survivors).
import 'package:test/test.dart';

import 'package:fixture_large/domain_02.dart';

void main() {
  test('f02_isPositive', () async {
    expect(f02_isPositive(3), isTrue); expect(f02_isPositive(-3), isFalse);
  });
  test('f02_isNonNegative covered only', () {
    f02_isNonNegative(0);
  });
  test('f02_isEqual', () async {
    expect(f02_isEqual(2, 2), isTrue); expect(f02_isEqual(2, 3), isFalse);
  });
}
