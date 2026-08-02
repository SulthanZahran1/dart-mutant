/// Fixture: small — one library covering the six Dart construct types.
///
/// Mutant budget (with the operators enabled in `.dart_mutant.yml`):
///   - AOR: `a + b` → 4 mutants (killed by exact-value assertions)
///   - ROR: `n > 0` → 5 mutants (killed by boundary assertions)
///   - LOR: `x && y` → 1 mutant (killed by the false/true pair)
///   - NullSafety: `value ?? defaultValue` → 1 mutant (killed by the null case)
///   - Cascade: `..removeLast()` → 1 mutant (killed by the dot-vs-cascade
///     type change — the mutant fails to compile/throws)
///   - AsyncAwait: `await _computeValue()` → 1 mutant (killed by the string
///     interpolation of the unresolved Future)
/// Total: 13 mutants, all killed → MSI 100%.
library fixture_small;

/// Arithmetic: AOR. `a + b` → `a - b`, `a * b`, `a / b`, `a % b`.
int add(int a, int b) => a + b;

/// Relational: ROR. `n > 0` → `<`, `>=`, `<=`, `==`, `!=`.
bool isStrictlyPositive(int n) => n > 0;

/// Logical: LOR. `x && y` → `x || y`.
bool both(bool x, bool y) => x && y;

/// Null-safety: NullSafety. `value ?? defaultValue` → `value`. The nullable
/// return type keeps the mutant compiling; the null-input assertion kills it.
String? fallback(String? value, String defaultValue) => value ?? defaultValue;

/// Cascade: Cascade. `..removeLast()` → `.removeLast()`. The mutant assigns
/// the removed element (an `int`) to the list variable, so `list.length`
/// fails to compile — killing the mutant.
int listLength() {
  final list = [1, 2, 3]
    ..removeLast();
  return list.length;
}

/// Async/await: AsyncAwait. `await _computeValue()` → `_computeValue()`,
/// leaving the unresolved Future in the interpolation.
Future<String> fetchLabel() async {
  final n = await _computeValue();
  return 'value: $n';
}

Future<int> _computeValue() async => 42;
