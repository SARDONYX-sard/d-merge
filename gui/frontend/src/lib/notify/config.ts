import { NOTIFY } from '@/lib/notify';
import { STORAGE } from '@/lib/storage';

import type { SnackbarOrigin } from 'notistack';

/**
 * Type representing the configuration for the notification system.
 */
type NotifyConfig = {
  /** Determines where the notification will appear on the screen. */
  anchorOrigin: SnackbarOrigin;
  /** Maximum number of snack_bars to display simultaneously. */
  maxSnack: number;
};

/**
 * Default notification configuration.
 */
const DEFAULT = {
  anchorOrigin: {
    horizontal: 'left',
    vertical: 'top',
  },
  maxSnack: 3,
} as const satisfies NotifyConfig;

/**
 * Utility function to normalize the position values for the Snackbar.
 * Ensures that the vertical and horizontal values fall within acceptable limits.
 *
 * @param param0 - The partial object containing vertical and horizontal position values.
 * @returns - The normalized Snackbar position.
 */
const normalize = ({ vertical, horizontal }: Partial<{ vertical: string; horizontal: string }>) => {
  const isValidVertical = (value?: string) => value === 'bottom' || value === 'top';
  const isValidHorizontal = (value?: string) => value === 'center' || value === 'right' || value === 'left';

  return {
    vertical: isValidVertical(vertical) ? vertical : DEFAULT.anchorOrigin.vertical,
    horizontal: isValidHorizontal(horizontal) ? horizontal : DEFAULT.anchorOrigin.horizontal,
  } as const satisfies SnackbarOrigin;
};

/**
 * Main configuration object for the notification system. This handles setting and retrieving
 * the notification configuration, including the anchor position and maximum snack count.
 */
export const NOTIFY_CONFIG = {
  /**
   * The default configuration.
   */
  default: DEFAULT,

  /**
   * Retrieves the current notification configuration from storage, falling back to defaults if necessary.
   *
   * @returns - The current notification configuration.
   */
  getOrDefault() {
    const anchorOrigin = (() => {
      const position: Partial<SnackbarOrigin> = NOTIFY.try(() => {
        const jsonStr = STORAGE.get('snackbar-position');
        return jsonStr ? JSON.parse(jsonStr) : {};
      });
      return normalize(position) satisfies NotifyConfig['anchorOrigin'];
    })();

    const maxSnack = (() => {
      const maxSnackCache = Number(STORAGE.get('snackbar-limit') ?? DEFAULT.maxSnack);
      return Number.isNaN(maxSnackCache) ? DEFAULT.maxSnack : maxSnackCache;
    })();

    return { anchorOrigin, maxSnack } as const satisfies NotifyConfig;
  },

  /**
   * Sub-object for managing the `anchorOrigin` configuration.
   */
  anchor: {
    /**
     * Sets the anchor position and stores it.
     *
     * @param value - The new anchor position.
     */
    set(value: NotifyConfig['anchorOrigin']) {
      STORAGE.set('snackbar-position', JSON.stringify(value));
    },

    /**
     * Parses a string in the format "vertical_horizontal" to set the anchor position.
     *
     * @param str - The position string.
     * @returns - The parsed anchor position.
     */
    fromStr(str: string): NotifyConfig['anchorOrigin'] {
      const [vertical, horizontal] = str.split('_');
      return normalize({ vertical, horizontal });
    },
  },

  /**
   * Sub-object for managing the maximum snack count configuration.
   */
  limit: {
    /**
     * Sets the maximum snack limit and stores it.
     *
     * value - The new maximum snack limit.
     */
    set(value: NotifyConfig['maxSnack']) {
      STORAGE.set('snackbar-limit', value.toString());
    },

    /**
     * Parses a string to set the maximum snack limit.
     *
     * @param str - The string representation of the snack limit.
     * @returns - The parsed snack limit.
     */
    fromStr(str: string): NotifyConfig['maxSnack'] {
      return Number(str) || 1;
    },
  },
} as const;
