import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';

import { type Cache, STORAGE } from '@/lib/storage';

import { useStorageState } from './useStorageState';

const mockKey = 'log-level' satisfies keyof Cache;

describe('useStorageState', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('should initialize with fallback state if no STORAGE value', () => {
    const { result } = renderHook(() => useStorageState<string>(mockKey, 'fallback'));

    expect(result.current[0]).toBe('fallback');
  });

  it('should initialize with STORAGE value if it exists', () => {
    STORAGE.set(mockKey, JSON.stringify('storedValue'));

    const { result } = renderHook(() => useStorageState<string>(mockKey, 'fallback'));

    expect(result.current[0]).toBe('storedValue');
  });

  it('should save new string value to STORAGE', () => {
    const { result } = renderHook(() => useStorageState<string>(mockKey, 'fallback'));

    act(() => {
      result.current[1]('newValue');
    });

    expect(result.current[0]).toBe('newValue');
    expect(STORAGE.get(mockKey)).toBe(JSON.stringify('newValue'));
  });

  it('should handle numbers correctly', () => {
    const { result } = renderHook(() => useStorageState<number>(mockKey, 42));

    act(() => {
      result.current; // Read only.
    });

    expect(result.current[0]).toBe(42); // Use fallback values since there are no items in STORAGE.
    expect(STORAGE.get(mockKey)).toBeNull(); // Nothing in STORAGE because it is currently read only.
  });

  it('should handle objects correctly', () => {
    const { result } = renderHook(() => useStorageState<{ theme: string }>(mockKey, { theme: 'light' }));

    act(() => {
      result.current[1]({ theme: 'dark' });
    });

    expect(result.current[0]).toEqual({ theme: 'dark' });
    expect(STORAGE.get(mockKey)).toBe(JSON.stringify({ theme: 'dark' }));
  });

  it('should not update STORAGE if the value is the same', () => {
    const { result } = renderHook(() => useStorageState<string>(mockKey, 'sameValue'));

    act(() => {
      result.current[1]('sameValue');
    });

    expect(STORAGE.get(mockKey)).toBe('"sameValue"');
  });

  it('should handle invalid JSON in STORAGE gracefully', () => {
    STORAGE.set(mockKey, 'invalidJSON');

    const { result } = renderHook(() => useStorageState<string>(mockKey, 'fallback'));

    // Should fall back to the provided state
    expect(result.current[0]).toBe('fallback');
  });
});
