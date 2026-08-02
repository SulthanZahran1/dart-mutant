import 'package:fixture_flutter/main.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('counter starts at zero', (WidgetTester tester) async {
    await tester.pumpWidget(const FixtureApp());

    expect(find.text('Count: 0'), findsOneWidget);
  });

  testWidgets('increment advances the counter by one', (WidgetTester tester) async {
    await tester.pumpWidget(const FixtureApp());

    await tester.tap(find.text('Increment'));
    await tester.pump();
    expect(find.text('Count: 1'), findsOneWidget);

    await tester.tap(find.text('Increment'));
    await tester.pump();
    expect(find.text('Count: 2'), findsOneWidget);
  });

  testWidgets('counter resets to zero at the limit of 3', (WidgetTester tester) async {
    await tester.pumpWidget(const FixtureApp());

    // Reach the limit: three increments → Count: 3.
    for (int i = 0; i < 3; i++) {
      await tester.tap(find.text('Increment'));
      await tester.pump();
    }
    expect(find.text('Count: 3'), findsOneWidget);

    // 3 >= 3 → reset fires → back to zero.
    await tester.tap(find.text('Reset at limit'));
    await tester.pump();
    expect(find.text('Count: 0'), findsOneWidget);
  });

  testWidgets('counter does not reset below the limit', (WidgetTester tester) async {
    await tester.pumpWidget(const FixtureApp());

    await tester.tap(find.text('Increment'));
    await tester.pump();
    expect(find.text('Count: 1'), findsOneWidget);

    // 1 >= 3 is false → no reset.
    await tester.tap(find.text('Reset at limit'));
    await tester.pump();
    expect(find.text('Count: 1'), findsOneWidget);
  });

  testWidgets('counter resets past the limit', (WidgetTester tester) async {
    await tester.pumpWidget(const FixtureApp());

    // Four increments → Count: 4 (kills ROR `==` which only fires at exactly 3).
    for (int i = 0; i < 4; i++) {
      await tester.tap(find.text('Increment'));
      await tester.pump();
    }
    expect(find.text('Count: 4'), findsOneWidget);

    // 4 >= 3 → reset fires → back to zero.
    await tester.tap(find.text('Reset at limit'));
    await tester.pump();
    expect(find.text('Count: 0'), findsOneWidget);
  });
}
