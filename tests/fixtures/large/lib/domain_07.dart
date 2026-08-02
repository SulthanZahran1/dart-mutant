/// Large fixture — domain_07. All 17 operators exercised.
library fixture_large_07;

String f07_pickByFlag(bool flag) => flag ? 'yes' : 'no';
int f07_clampTo(int v, int lo, int hi) { if (v < lo) { return lo; } if (v > hi) { return hi; } return v; }
String f07_describe(int? v) { if (v == null) { return 'none'; } return 'v=$v'; }
