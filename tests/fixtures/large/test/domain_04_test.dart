// Large fixture test — domain_04. Asserts on ~2/3 of
// functions; the rest are deliberately unasserted (survivors).
import 'package:test/test.dart';

import 'package:fixture_large/domain_04.dart';

void main() {
  test('f04_grade', () async {
    expect(f04_grade(95), 'A'); expect(f04_grade(85), 'B'); expect(f04_grade(70), 'C');
  });
  test('f04_sumUpTo covered only', () {
    f04_sumUpTo(5); f04_sumUpTo(0);
  });
  test('f04_countDown', () async {
    expect(f04_countDown(4), 4);
  });
}
