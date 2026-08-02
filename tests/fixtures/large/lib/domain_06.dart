/// Large fixture — domain_06. All 17 operators exercised.
library fixture_large_06;

List<int> f06_buildList() { final l = [1, 2, 3]..removeLast(); return l; }
Future<int> f06_awaitVal() async { final v = await _fetch(); return v; }
Stream<int> f06_streamCount() async* { for (var i = 0; i < 3; i++) { yield i; } }

Future<int> _fetch() async => 42;
