import 'package:flutter/material.dart';

void main() {
  runApp(const FixtureApp());
}

/// Minimal Flutter app fixture for dart_mutant.
///
/// Deliberately small: a counter with real logic — an arithmetic expression
/// (AOR/AOD/AOI mutation points) and a conditional (ROR/COR mutation points) —
/// covered by widget tests in `test/widget_test.dart`.
class FixtureApp extends StatelessWidget {
  const FixtureApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Fixture Flutter',
      home: const CounterPage(),
    );
  }
}

/// Stateful counter page — the mutation target.
class CounterPage extends StatefulWidget {
  const CounterPage({super.key});

  @override
  State<CounterPage> createState() => _CounterPageState();
}

class _CounterPageState extends State<CounterPage> {
  int _count = 0;

  /// Increment the counter by one.
  ///
  /// Mutation points: `_count + 1` — AOR (`+` → `-`/`*`/`/`/`%`), AOD
  /// (`a + b` → `a`), AOI (`a` → `a + 1`), SDL (delete the statement).
  void _increment() {
    setState(() {
      _count = _count + 1;
    });
  }

  /// Reset the counter to zero once it reaches the limit of 3.
  ///
  /// Mutation points: `_count >= 3` — ROR (`>=` → `>`/`<`/`<=`/`==`/`!=`),
  /// COR (negate the condition), SDL (delete the assignment or the `if`).
  void _resetAtLimit() {
    if (_count >= 3) {
      setState(() {
        _count = 0;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Counter')),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: <Widget>[
            Text('Count: $_count'),
            const SizedBox(height: 16),
            ElevatedButton(
              onPressed: _increment,
              child: const Text('Increment'),
            ),
            const SizedBox(height: 8),
            ElevatedButton(
              onPressed: _resetAtLimit,
              child: const Text('Reset at limit'),
            ),
          ],
        ),
      ),
    );
  }
}
