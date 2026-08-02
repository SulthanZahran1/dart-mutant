// Large fixture test — domain_06. Asserts on ~2/3 of
// functions; the rest are deliberately unasserted (survivors).
import 'package:test/test.dart';

import 'package:fixture_large/domain_06.dart';

void main() {
  test('f06_buildList', () async {
    expect(f06_buildList(), [1, 2]);
  });
  test('f06_awaitVal covered only', () {
    f06_awaitVal();
  });
  test('f06_streamCount', () async {
    expect(await f06_streamCount().toList(), [0, 1, 2]);
  });
}
