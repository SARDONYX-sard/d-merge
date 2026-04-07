import { enqueueSnackbar } from 'notistack';
import { LOG, LogLevel } from '@/services/api/log';

import type { OptionsObject, SnackbarMessage } from 'notistack';

/**
 * Wrapper to simplify refactoring of libraries such as snackbar and toast
 */
export const NOTIFY = {
  /** Show as `info` message. */
  info(message: SnackbarMessage, options?: OptionsObject<'info'>) {
    logString('info', message);
    return enqueueSnackbar(message, { variant: 'info', ...options });
  },
  /** Show as `success` message. */
  success(message: SnackbarMessage, options?: OptionsObject<'success'>) {
    logString('info', message);
    return enqueueSnackbar(message, { variant: 'success', ...options });
  },
  /** Show as `warning` message. */
  warn(message: SnackbarMessage, options?: OptionsObject<'warning'>) {
    logString('warn', message);
    return enqueueSnackbar(message, { variant: 'warning', ...options });
  },
  /** Show as `error` message. */
  error(message: SnackbarMessage, options?: OptionsObject<'error'>) {
    logString('error', message);
    return enqueueSnackbar(message, { variant: 'error', ...options });
  },

  /** Try to execute function, and then catch & notify if error. */
  try<Fn extends () => ReturnType<Fn>>(tryFn: Fn): ReturnType<Fn> | undefined {
    try {
      return tryFn();
    } catch (error) {
      if (error instanceof Error) {
        NOTIFY.error(error.message);
      }
    }
  },

  /** Try to execute async function, and then catch & notify if error. */
  async asyncTry<Args extends any[], R>(tryFn: (...args: Args) => Promise<R>, ...args: Args) {
    tryFn(...args)
      .catch((e) => {
        NOTIFY.error(`${e}`);
        return undefined;
      })
      .catch((e) => console.error('Unexpected error in NOTIFY.asyncTry:', e));
  },
} as const;

const logString = (logLevel: LogLevel, message: SnackbarMessage) => {
  let messageStr = null;
  if (typeof message === 'string') messageStr = message;
  if (typeof message === 'number') messageStr = String(message);

  if (messageStr !== null) LOG.log(logLevel, messageStr);
};
