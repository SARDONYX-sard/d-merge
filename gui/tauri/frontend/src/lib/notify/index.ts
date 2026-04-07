import { enqueueSnackbar } from 'notistack';
import { LOG } from '@/services/api/log';

import type { OptionsObject, SnackbarMessage } from 'notistack';

/**
 * Wrapper to simplify refactoring of libraries such as snackbar and toast
 */
export const NOTIFY = {
  /** Show as `info` message. */
  info(message: SnackbarMessage, options?: OptionsObject<'info'>) {
    LOG.log('info', `${message}`);
    return enqueueSnackbar(message, { variant: 'info', ...options });
  },
  /** Show as `success` message. */
  success(message: SnackbarMessage, options?: OptionsObject<'success'>) {
    LOG.log('info', `${message}`);
    return enqueueSnackbar(message, { variant: 'success', ...options });
  },
  /** Show as `warning` message. */
  warn(message: SnackbarMessage, options?: OptionsObject<'warning'>) {
    LOG.log('warn', `${message}`);
    return enqueueSnackbar(message, { variant: 'warning', ...options });
  },
  /** Show as `error` message. */
  error(message: SnackbarMessage, options?: OptionsObject<'error'>) {
    LOG.log('error', `${message}`);
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
  async asyncTry<Args extends any[], R>(tryFn: (...args: Args) => Promise<R>, ...args: Args): Promise<R | undefined> {
    return tryFn(...args).catch((e) => {
      NOTIFY.error(`${e}`);
      return undefined;
    });
  },
} as const;
