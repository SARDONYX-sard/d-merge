'use client';

import { type Resource, use } from 'i18next';
import { initReactI18next } from 'react-i18next';

import { NOTIFY } from '@/lib/notify';
import { STORAGE } from '@/lib/storage';

import dictEnUs from '@/../../locales/en-US.json';
import dictJaJp from '@/../../locales/ja-JP.json';

/** The keys in RESOURCE are language tags according to the BCP-47 standard.
    - See: https://partnerhub.warnermediagroup.com/metadata/languages */
const RESOURCES = {
  'en-US': {
    translation: dictEnUs,
  },
  'ja-JP': {
    translation: dictJaJp,
  },
  custom: { translation: NOTIFY.try(() => JSON.parse(STORAGE.get('custom-translation-dict') ?? '{}')) },
} as const satisfies Resource;

/**
 * Default if `null` or `undefined`.
 * @default `en-US`
 */
const normalize = (str: string | null): Exclude<I18n, 'auto'> => {
  switch (str === null || str === 'auto' ? window.navigator.language : str) {
    case 'ja':
    case 'ja-JP':
      return 'ja-JP';
    case 'custom':
      return 'custom';
    default:
      return 'en-US';
  }
};

type I18n = 'auto' | keyof typeof RESOURCES;
export type I18nKeys = keyof (typeof RESOURCES)['en-US']['translation'];

const CACHE_KEY = 'locale';
export const LANG = {
  /**
   * NOTE: This is intended to be done automatically the moment global import is done each time on the previous page in `src/app/layout.tsx`
   * - ref: https://react.i18next.com/guides/quick-start#configure-i18next
   */
  init() {
    use(initReactI18next) // passes i18n down to react-i18next
      .init({
        resources: RESOURCES,
        lng: normalize(STORAGE.get(CACHE_KEY)),
        fallbackLng: 'en-US',
        interpolation: {
          escapeValue: false, // react already safes from xss
        },
      });
  },

  normalize,

  /** get current log level from `LocalStorage`. */
  get(): I18n {
    const locale = STORAGE.get(CACHE_KEY);
    return locale === 'auto' ? 'auto' : normalize(locale);
  },

  /** set log level to `LocalStorage`. */
  set(lang: I18n) {
    STORAGE.set(CACHE_KEY, lang);
  },
} as const;
