import type { z } from 'zod';

/** @see https://zenn.dev/ynakamura/articles/65d58863563fbc#%E8%A7%A3%E6%B1%BA%E7%AD%962%EF%BC%88recommended%EF%BC%89-schemefortype-utility-function%E3%82%92%E5%AE%9A%E7%BE%A9%E3%81%99%E3%82%8B */
export const schemaForType =
  <T>() =>
  <S extends z.ZodType<T, any, any>>(arg: S) => {
    return arg;
  };
