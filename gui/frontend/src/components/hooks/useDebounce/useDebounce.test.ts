import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';

import { useDebounce } from './useDebounce';

describe('useDebounce', () => {
  vi.useFakeTimers();

  it('should return the initial value immediately', () => {
    const { result } = renderHook(() => useDebounce('test', 500));
    expect(result.current).toBe('test');
  });

  it('should debounce the value', () => {
    vi.useFakeTimers(); // Setup functions required to use `advanceTimersByTime`.
    const { result, rerender } = renderHook(({ value, delay }) => useDebounce(value, delay), {
      initialProps: { value: 'initial', delay: 500 },
    });

    expect(result.current).toBe('initial');

    rerender({ value: 'updated', delay: 500 });

    // The value should not change immediately
    expect(result.current).toBe('initial');

    act(() => {
      vi.advanceTimersByTime(500);
    });

    expect(result.current).toBe('updated');
  });

  it('should reset the timer when the value changes quickly', () => {
    const { result, rerender } = renderHook(({ value, delay }) => useDebounce(value, delay), {
      initialProps: { value: 'initial', delay: 500 },
    });

    expect(result.current).toBe('initial');

    // Update the value multiple times
    rerender({ value: 'first update', delay: 500 });
    act(() => {
      vi.advanceTimersByTime(200);
    });
    rerender({ value: 'second update', delay: 500 });
    act(() => {
      vi.advanceTimersByTime(200);
    });

    // The value should still be the initial value
    expect(result.current).toBe('initial');

    // Fast-forward time by 500ms
    act(() => {
      vi.advanceTimersByTime(500);
    });

    // Now the value should update to the latest one
    expect(result.current).toBe('second update');
  });

  vi.useRealTimers();
});
