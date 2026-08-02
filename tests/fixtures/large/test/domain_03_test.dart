// Large fixture test — domain_03. Asserts on ~2/3 of
// functions; the rest are deliberately unasserted (survivors).
import 'package:test/test.dart';

import 'package:fixture_large/domain_03.dart';

void main() {
  test('f03_isNotEqual', () async {
    expect(f03_isNotEqual(2, 3), isTrue);
  });
  test('f03_bothTrue covered only', () {
    f03_bothTrue(true, true); f03_bothTrue(true, false);
  });
  test('f03_eitherTrue', () async {
    expect(f03_eitherTrue(false, true), isTrue); expect(f03_eitherTrue(false, false), isFalse);
  });
}
