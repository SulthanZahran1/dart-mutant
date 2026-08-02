import 'package:test/test.dart';
import '../lib/calculator.dart';

void main() {
  test('add sums two integers', () {
    expect(add(2, 3), equals(5));
    expect(add(-1, 1), equals(0));
  });

  test('isEven detects parity', () {
    expect(isEven(4), isTrue);
    expect(isEven(7), isFalse);
  });

  test('sumUpTo sums 0..n-1 in bounded time', () {
    expect(sumUpTo(10), equals(45));
    expect(sumUpTo(0), equals(0));
  });

  test('inRange checks inclusive bounds', () {
    expect(inRange(5, 1, 10), isTrue);
    expect(inRange(0, 1, 10), isFalse);
    expect(inRange(11, 1, 10), isFalse);
    expect(inRange(1, 1, 10), isTrue);
    expect(inRange(10, 1, 10), isTrue);
  });
}
