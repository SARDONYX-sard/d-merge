import { type ReactNode, createContext, useContext, useState } from 'react';

import { STORAGE } from '@/lib/storage';
import { PUB_CACHE_OBJ } from '@/lib/storage/cacheKeys';

type ContextType = {
  js: string;
  setJs: (value?: string) => void;
};
const Context = createContext<ContextType | undefined>(undefined);
const CACHE_KEY = PUB_CACHE_OBJ.customJs;

type Props = { children: ReactNode };

/** Wrapper component to allow user-defined css and existing css design presets to be retrieved/modified from anywhere */
export const JsProvider = ({ children }: Props) => {
  const [js, setJs] = useState(STORAGE.get(CACHE_KEY) ?? '');

  const setHook = (value?: string) => {
    if (value) {
      setJs(value);
      STORAGE.set(CACHE_KEY, value);
    } else {
      STORAGE.remove(CACHE_KEY);
    }
  };

  return <Context.Provider value={{ js, setJs: setHook }}>{children}</Context.Provider>;
};

/**
 * @throws `useJsContext must be used within a JsProvider`
 */
export const useJsContext = () => {
  const context = useContext(Context);
  if (!context) {
    throw new Error('useJsContext must be used within a JsProvider');
  }
  return context;
};
