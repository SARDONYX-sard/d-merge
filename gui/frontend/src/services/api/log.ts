import { app } from '@tauri-apps/api';
import { invoke } from '@tauri-apps/api/core';
import { appLogDir } from '@tauri-apps/api/path';
import { open } from '@tauri-apps/plugin-shell';

import { STORAGE } from '@/lib/storage';

export type LogLevel = 'trace' | 'debug' | 'info' | 'warn' | 'error';

const DEFAULT = 'error';
const CACHE_KEY = 'log-level';

/**
 * `'error'` if null or undefined
 * @default `error`
 */
const normalize = (logLevel?: string | null): LogLevel => {
  switch (logLevel) {
    case 'trace':
    case 'debug':
    case 'info':
    case 'warn':
    case 'error':
      return logLevel;
    default:
      return DEFAULT;
  }
};

export const LOG = {
  default: DEFAULT,

  /**
   * Opens the log file.
   * @throws - if not found path
   */
  async openFile() {
    const logFile = `${await appLogDir()}/${await app.getName()}.log`;
    await open(logFile);
  },

  /**
   * Opens the log directory.
   * @throws - if not found path
   */
  async openDir() {
    await open(await appLogDir());
  },

  /**
   * Invokes the `change_log_level` command with the specified log level.
   * @param logLevel - The log level to set. If not provided, the default log level will be used.
   * @returns A promise that resolves when the log level is changed.
   */
  async changeLevel(logLevel?: LogLevel) {
    await invoke('change_log_level', { logLevel });
  },

  normalize,

  /** get current log level from `LocalStorage`. */
  get() {
    return normalize(STORAGE.get(CACHE_KEY));
  },

  /** set log level to `LocalStorage`. */
  set(level: LogLevel) {
    STORAGE.set(CACHE_KEY, level);
  },
} as const;
