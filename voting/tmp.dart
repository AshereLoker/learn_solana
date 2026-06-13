class Balance {
  int _value;

  Balance([int initial = 0]) : _value = initial;

  int get value => _value;

  /// Записывает новое значение (аналог присвоения int).
  void write(int newValue) => _value = newValue;

  @override
  String toString() => 'Balance($_value)';
}

// ─── глобальный объект вместо голого int ───────────────────────────────────
Balance balance = Balance();

Future<void> deposit(String who, int amount, Duration ioDelay) async {
  final current = balance; // read
  await Future<void>.delayed(ioDelay); // simulated network/db call -> YIELD
  balance.write(current.value + amount); // write back a STALE value
  print('  [$who] read=$current, wrote=${balance.value}');
}

Future<void> raceLostUpdate() async {
  print('Race #1: lost update across await gap');
  balance = Balance(0);

  await Future.wait([
    deposit('task A', 100, const Duration(milliseconds: 50)),
    deposit('task B', 100, const Duration(milliseconds: 100)),
  ]);

  print('  expected balance: 200');
  print(
    '  actual   balance: ${balance.value}   '
    '${balance.value == 200 ? "OK" : "<-- RACE: update lost!"}\n',
  );
}

Future<void> main() async {
  await raceLostUpdate();
}
