/// Medium fixture — calculator domain.
library fixture_medium_calculator;

/// AOR: `a + b` → 4 arithmetic replacements.
int add(int a, int b) => a + b;

/// AOR (on `%`) + ROR (on `==`).
bool isEven(int n) => n % 2 == 0;

/// INC: `i++` → `i--` makes this loop infinite for n > 0 → TIMEOUT.
/// ROR: `i < n` → 5 replacements (`i != n` survives — same iteration count).
/// SDL: `sum += i;` deleted → returns 0 → killed.
int sumUpTo(int n) {
  var sum = 0;
  for (var i = 0; i < n; i++) {
    sum += i;
  }
  return sum;
}

/// ROR: 2 conditions × 5 replacements; `>=` → `>` and `!=` survive for
/// boundary-tested inputs. COR: 2 negated-condition mutants.
String grade(int score) {
  if (score >= 90) {
    return 'A';
  }
  if (score >= 80) {
    return 'B';
  }
  return 'C';
}

/// LOR (`&&` → `||`) + ROR (both conditions).
bool inRange(int v, int lo, int hi) => v >= lo && v <= hi;
