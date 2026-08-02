// Large fixture test — domain_00. Asserts on ~2/3 of
// functions; the rest are deliberately unasserted (survivors).
import 'package:test/test.dart';

import 'package:fixture_large/domain_00.dart';

void main() {
  test('f00_add', () async {
    expect(f00_add(2, 3), 5); expect(f00_add(-1, 1), 0);
  });
  test('f00_sub covered only', () {
    f00_sub(5, 3);
  });
  test('f00_mul', () async {
    expect(f00_mul(4, 3), 12);
  });
}
