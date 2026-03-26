/** Schedules `fn` after `ms` of quiet time; each call resets the timer. */
export function debounce(fn: () => void, ms: number): () => void {
  let timeout: ReturnType<typeof setTimeout> | null = null;
  return () => {
    if (timeout) clearTimeout(timeout);
    timeout = setTimeout(() => {
      timeout = null;
      fn();
    }, ms);
  };
}

export function throttle<T extends unknown[]>(
  fn: (...args: T) => void,
  ms: number,
): (...args: T) => void {
  let last = 0;
  let timeout: ReturnType<typeof setTimeout> | null = null;
  let lastArgs: T | null = null;

  return (...args: T) => {
    const now = Date.now();
    const remaining = ms - (now - last);
    lastArgs = args;
    if (remaining <= 0) {
      if (timeout) {
        clearTimeout(timeout);
        timeout = null;
      }
      last = now;
      fn(...args);
      return;
    }
    if (timeout) return;
    timeout = setTimeout(() => {
      timeout = null;
      last = Date.now();
      if (lastArgs) fn(...lastArgs);
    }, remaining);
  };
}
