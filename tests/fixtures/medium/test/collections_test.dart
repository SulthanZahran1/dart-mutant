import 'package:test/test.dart';
import '../lib/collections.dart';

void main() {
  test('doubleValues doubles each element and keeps list type', () {
    final result = doubleValues([1, 2, 3]);
    expect(result, isA<List<int>>());
    expect(result, orderedEquals([2, 4, 6]));
  });

  test('buildList cascades adds onto the same list', () {
    expect(buildList(), orderedEquals([1, 2]));
  });

  test('anyPositive detects a positive element', () {
    expect(anyPositive([1, -2]), isTrue);
    expect(anyPositive([-1, -2]), isFalse);
  });
}
