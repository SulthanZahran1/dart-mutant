import 'package:test/test.dart';
import '../lib/fixture_small.dart';

void main() {
  test('add returns the exact sum', () {
    // Kills all four AOR mutants: -, *, /, % all give != 5 for (2, 3).
    expect(add(2, 3), equals(5));
    expect(add(0, 0), equals(0));
    expect(add(-1, 1), equals(0));
  });

  test('isStrictlyPositive handles positives, zero, and negatives', () {
    // (5) kills <, <=, ==; (0) kills >=; (-1) kills !=.
    expect(isStrictlyPositive(5), isTrue);
    expect(isStrictlyPositive(0), isFalse);
    expect(isStrictlyPositive(-1), isFalse);
  });

  test('both requires both operands to be true', () {
    // (true, false) -> false kills the || mutant.
    expect(both(true, true), isTrue);
    expect(both(true, false), isFalse);
    expect(both(false, true), isFalse);
  });

  test('fallback uses the value when present and the default when null', () {
    expect(fallback('hello', 'world'), equals('hello'));
    // Kills the ??-removal mutant: mutated code returns null here.
    expect(fallback(null, 'world'), equals('world'));
  });

  test('listLength removes the last element via cascade', () {
    // Kills the cascade mutant: `.removeLast()` returns 3 (an int), so
    // `list.length` throws NoSuchMethodError instead of returning 2.
    expect(listLength(), equals(2));
  });

  test('fetchLabel awaits the computed value', () async {
    // Kills the await-removal mutant: the string interpolates the
    // unresolved Future instead of the value 42.
    expect(await fetchLabel(), equals('value: 42'));
  });
}
