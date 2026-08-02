/// Medium fixture — async utilities.
library fixture_medium_async;

/// AsyncAwait: removing `await` in `async => await f()` is flattened by the
/// async runtime → same value → SURVIVED.
Future<int> fetchValue() async => await _computeValue();

/// AsyncAwait: removing `await` leaves a Future in the interpolation →
/// different string → KILLED.
Future<String> fetchName() async {
  final name = await _getName();
  return 'Hello $name';
}

/// StreamMutation: `.first` → `.last` → killed by first-element assertion.
/// AsyncAwait: flattened → SURVIVED.
Future<int> firstValue(Stream<int> stream) async => await stream.first;

/// StreamMutation: `.toList()` → `.toSet()` → killed by list equality.
Future<List<int>> collect(Stream<int> stream) async => await stream.toList();

Future<int> _computeValue() async => 42;

Future<String> _getName() async => 'Alice';
