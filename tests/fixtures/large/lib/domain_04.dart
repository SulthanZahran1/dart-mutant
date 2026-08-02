/// Large fixture — domain_04. All 17 operators exercised.
library fixture_large_04;

String f04_grade(int s) { if (s >= 90) { return 'A'; } if (s >= 80) { return 'B'; } return 'C'; }
int f04_sumUpTo(int n) { var s = 0; for (var i = 0; i < n; i++) { s += i; } return s; }
int f04_countDown(int n) { var c = 0; while (n > 0) { n--; c++; } return c; }
