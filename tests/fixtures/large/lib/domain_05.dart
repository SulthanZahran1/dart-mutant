/// Large fixture — domain_05. All 17 operators exercised.
library fixture_large_05;

String f05_pickNonNull(String? a, String b) => a ?? b;
int f05_assertNonNull(int? v) { return v!; }
String f05_trimmed(String? s) => s?.trim() ?? '';
