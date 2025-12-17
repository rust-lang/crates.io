const COUNTERS = new Map();

export function increment(counterName) {
  let value = (COUNTERS.get(counterName) || 0) + 1;
  COUNTERS.set(counterName, value);
  return value;
}

export function reset() {
  COUNTERS.clear();
}
