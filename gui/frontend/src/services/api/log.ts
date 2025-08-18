import { app } from '@tauri-apps/api';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { appLogDir as tauriAppLogDir } from '@tauri-apps/api/path';
import { z } from 'zod';
import { STORAGE } from '@/lib/storage';
import { PUB_CACHE_OBJ } from '@/lib/storage/cacheKeys';
import { stringToJsonSchema } from '@/lib/zod/json-validation';
import { electronApi, isElectron } from '@/services/api/electron/setup';
import { openPath } from '@/services/api/shell';

const logList = ['trace', 'debug', 'info', 'warn', 'error'] as const;
const DEFAULT = 'error';
const logLevelSchema = z.enum(logList).catch(DEFAULT);
export type LogLevel = z.infer<typeof logLevelSchema>;

/** @default `error` */
const normalize = (logLevel?: string | null): LogLevel => {
  return logLevelSchema.parse(logLevel);
};

export const LOG = {
  /**
   * Opens the log file.
   * @throws - if not found path
   */
  async openFile() {
    const logFile = `${await getLogDir()}/${await getAppName()}.log`;
    await openPath(logFile);
  },

  /**
   * Opens the log directory.
   * @throws - if not found path
   */
  async openDir() {
    await openPath(await getLogDir());
  },

  /**
   * Invokes the `change_log_level` command with the specified log level.
   * @param logLevel - The log level to set. If not provided, the default log level will be used.
   * @returns A promise that resolves when the log level is changed.
   */
  async changeLevel(logLevel?: LogLevel) {
    if (isTauri()) {
      await invoke('change_log_level', { logLevel });
    } else if (isElectron()) {
      await electronApi.changeLogLevel(logLevel);
    } else {
      throw new Error('Unsupported platform: Neither Tauri nor Electron');
    }
  },

  normalize,

  /** get current log level from `LocalStorage`. */
  get() {
    return stringToJsonSchema.catch('error').pipe(logLevelSchema).parse(STORAGE.get(PUB_CACHE_OBJ.logLevel));
  },

  /** set log level to `LocalStorage`. */
  set(level: LogLevel) {
    STORAGE.set(PUB_CACHE_OBJ.logLevel, JSON.stringify(level));
  },
} as const;

async function getLogDir(): Promise<string> {
  if (isTauri()) {
    return await tauriAppLogDir();
  } else if (isElectron()) {
    return await electronApi.getAppLogDir();
  }
  throw new Error('Unsupported platform: Neither Tauri nor Electron');
}

async function getAppName(): Promise<string> {
  if (isTauri()) {
    return await app.getName();
  } else if (isElectron()) {
    return await electronApi.getAppName();
  }
  throw new Error('Unsupported platform: Neither Tauri nor Electron');
}
