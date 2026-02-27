import { withAnimation } from "./animation.js";

/**
 * Run an async function with a loading animation.
 * Silently skipped when `json` is truthy (pure JSON output) or non-TTY.
 */
export async function withSpinner<T>(
  _label: string,
  fn: () => Promise<T>,
  json?: boolean,
): Promise<T> {
  if (json) {
    return fn();
  }
  return withAnimation(fn);
}
