/// Medium fixture — collection utilities.
library fixture_medium_collections;

/// AOR: `v * 2` → 4 replacements, killed by exact list equality.
/// StreamMutation: `.toList()` → `.toSet()` → killed by ordered equality.
List<int> doubleValues(List<int> values) => values.map((v) => v * 2).toList();

/// Cascade: `..add(1)` → `.add(1)` is a compile error (void receiver);
/// `..add(2)` → `.add(2)` still cascades on the list → SURVIVED.
List<int> buildList() {
  final list = <int>[];
  list
    ..add(1)
    ..add(2);
  return list;
}

/// ROR: `v > 0` → 5 replacements (`>=` survives for tested inputs).
/// COR + LCR: killed by the mixed-input assertions.
bool anyPositive(List<int> values) {
  for (final v in values) {
    if (v > 0) {
      return true;
    }
  }
  return false;
}
