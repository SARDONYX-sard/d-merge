import { type ReactNode, createContext, useContext, useState } from 'react';

import { STORAGE } from '@/lib/storage';
import { PUB_CACHE_OBJ } from '@/lib/storage/cacheKeys';

type TabPosition = 'top' | 'bottom';
type ContextType = {
  tabPos: TabPosition;
  setTabPos: (value?: string) => void;
};
const Context = createContext<ContextType | undefined>(undefined);

const normalize = (value: string | null) => (value === 'bottom' ? 'bottom' : 'top');

type Props = { children: ReactNode };
export const TabProvider = ({ children }: Props) => {
  const [tabPos, setTabPos] = useState<TabPosition>(normalize(STORAGE.get(PUB_CACHE_OBJ.settingsTabPosition)));

  const setHook = (value?: string) => {
    if (value) {
      const validValue = normalize(value);
      setTabPos(validValue);
      STORAGE.set(PUB_CACHE_OBJ.settingsTabPosition, validValue);
    } else {
      STORAGE.remove(PUB_CACHE_OBJ.settingsTabPosition);
    }
  };

  return <Context.Provider value={{ tabPos, setTabPos: setHook }}>{children}</Context.Provider>;
};

/**
 * @throws `useTabContext must be used within a TabProvider`
 */
export const useTabContext = () => {
  const context = useContext(Context);
  if (!context) {
    throw new Error('useJsContext must be used within a TabProvider');
  }
  return context;
};
