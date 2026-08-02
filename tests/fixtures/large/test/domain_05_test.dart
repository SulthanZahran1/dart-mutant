// Large fixture test — domain_05. Asserts on ~2/3 of
// functions; the rest are deliberately unasserted (survivors).
import 'package:test/test.dart';

import 'package:fixture_large/domain_05.dart';

void main() {
  test('f05_pickNonNull', () async {
    expect(f05_pickNonNull(null, 'b'), 'b'); expect(f05_pickNonNull('a', 'b'), 'a');
  });
  test('f05_assertNonNull covered only', () {
    f05_assertNonNull(5);
  });
  test('f05_trimmed', () async {
    expect(f05_trimmed('  x  '), 'x'); expect(f05_trimmed(''), '');
  });
}
