import { z } from 'zod';
import { type CacheKey, STORAGE } from '@/lib/storage';
import { Json, stringToJsonSchema } from '@/lib/zod/json-validation';

/**
 * Provides methods for interacting with a storage system with schema validation.
 *
 * NOTE: Use `useStorageState` if you rely on `React.useState`.
 */
export const schemaStorage = {
  /**
   * Retrieves and validates data from storage.
   *
   * @template T - The type of the data.
   * @param key - The key to retrieve the data from storage.
   * @param schema - The Zod schema used for validation.
   * @returns The parsed data if valid, otherwise `null`.
   */
  get<T>(key: CacheKey, schema: z.ZodType<T, Json>): T | null {
    const data = STORAGE.get(key);
    if (data === null) {
      return null;
    }

    const result = stringToJsonSchema.catch(null).pipe(schema).safeParse(data);

    if (result.success) {
      return result.data;
    }
    return null;
  },

  /**
   * Stores data in storage as a JSON string.
   *
   * @template The type of the value to be stored.
   * @param key - The key to store the data under.
   * @param value - The value to store.
   */
  set<T>(key: CacheKey, value: T): void {
    STORAGE.set(key, JSON.stringify(value));
  },

  /**
   * Retrieves and validates data from storage, and returns it along with a function to update the value.
   *
   * @template The type of the data.
   * @param key - The key to retrieve the data from storage.
   * @param schema - The Zod schema used for validation.
   * @returns A tuple containing the parsed data and a function to set the value.
   */
  use<T>(key: CacheKey, schema: z.ZodType<T, Json>): [T | null, (value: T) => void] {
    const value = this.get(key, schema);
    const setValue = (newValue: T) => {
      this.set(key, newValue);
    };

    return [value, setValue];
  },
};
