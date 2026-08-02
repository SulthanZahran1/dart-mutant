import 'package:test/test.dart';
import '../lib/async_utils.dart';

void main() {
  test('fetchValue resolves to 42', () async {
    expect(await fetchValue(), equals(42));
  });

  test('fetchName greets the fetched name', () async {
    expect(await fetchName(), equals('Hello Alice'));
  });

  test('firstValue takes the first stream element', () async {
    expect(await firstValue(Stream.fromIterable([1, 2, 3])), equals(1));
  });

  test('collect gathers all stream elements into a list', () async {
    final result = await collect(Stream.fromIterable([1, 2, 3]));
    expect(result, isA<List<int>>());
    expect(result, orderedEquals([1, 2, 3]));
  });
}
