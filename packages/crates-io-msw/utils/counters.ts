const COUNTERS = new Map<string, number>();

export function increment(counterName: string) {
  let value = (COUNTERS.get(counterName) || 0) + 1;
  COUNTERS.set(counterName, value);
  return value;
}

export function reset() {
  COUNTERS.clear();
}
