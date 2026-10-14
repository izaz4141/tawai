import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:tawai/ui/widgets/components/list_slider.dart';

void main() {
  Future<Slider> pumpSlider<T extends num>(
    WidgetTester tester,
    ValueNotifier<T> notifier, {
    required T min,
    required T max,
    T? step,
    T? defaultValue,
    ListSliderEnd<T>? minValue,
    ListSliderEnd<T>? maxValue,
    void Function(T)? onChanged,
    void Function(T)? onChangeEnd,
  }) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ListSlider<T>(
            title: 't',
            valueListenable: notifier,
            min: min,
            max: max,
            step: step,
            defaultValue: defaultValue,
            minValue: minValue,
            maxValue: maxValue,
            onChanged: onChanged ?? (v) => notifier.value = v,
            onChangeEnd: onChangeEnd,
          ),
        ),
      ),
    );
    return tester.widget<Slider>(find.byType(Slider));
  }

  testWidgets('int slider snaps to the step grid and stays an int', (
    tester,
  ) async {
    final n = ValueNotifier<int>(0);
    final slider = await pumpSlider<int>(tester, n, min: 0, max: 10);

    slider.onChanged!(3.4);
    expect(n.value, 3);
    expect(n.value, isA<int>());

    slider.onChanged!(3.6);
    expect(n.value, 4);
  });

  testWidgets('double slider keeps double and honors a fractional step', (
    tester,
  ) async {
    final n = ValueNotifier<double>(0);
    final slider = await pumpSlider<double>(
      tester,
      n,
      min: 0,
      max: 1,
      step: 0.1,
    );

    slider.onChanged!(0.34);
    expect(n.value, isA<double>());
    expect(n.value, closeTo(0.3, 1e-9));

    slider.onChanged!(0.66);
    expect(n.value, closeTo(0.7, 1e-9));
  });

  testWidgets('sentinel max end past the numeric range is reachable', (
    tester,
  ) async {
    final n = ValueNotifier<int>(32);
    final slider = await pumpSlider<int>(
      tester,
      n,
      min: 32,
      max: 320,
      maxValue: (label: 'Lossless', value: 321),
    );

    expect(find.text('Lossless'), findsOneWidget);
    slider.onChanged!(321.0);
    expect(n.value, 321);
  });

  testWidgets('onChangeEnd fires on drag end', (tester) async {
    final n = ValueNotifier<int>(5);
    final commits = <int>[];
    final slider = await pumpSlider<int>(
      tester,
      n,
      min: 0,
      max: 10,
      onChanged: (v) => n.value = v,
      onChangeEnd: commits.add,
    );

    slider.onChangeEnd!(7.0);
    expect(commits, [7]);
  });

  testWidgets('reset button previews and commits the default value', (
    tester,
  ) async {
    final n = ValueNotifier<int>(5);
    final commits = <int>[];
    await pumpSlider<int>(
      tester,
      n,
      min: 0,
      max: 10,
      defaultValue: 0,
      onChanged: (v) => n.value = v,
      onChangeEnd: commits.add,
    );

    expect(find.byIcon(Icons.arrow_back), findsOneWidget);
    await tester.tap(find.byIcon(Icons.arrow_back));
    await tester.pump();

    expect(n.value, 0);
    expect(commits, [0]);
  });
}
