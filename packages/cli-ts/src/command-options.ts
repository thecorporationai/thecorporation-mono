export function inheritOption<T>(localValue: T | undefined, parentValue: T | undefined): T | undefined {
  return localValue ?? parentValue;
}
