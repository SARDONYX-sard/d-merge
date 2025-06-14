import { CssProvider } from '@/components/providers/CssProvider';
import { EditorModeProvider } from '@/components/providers/EditorModeProvider';
import { JsProvider } from '@/components/providers/JsProvider';
import { LogLevelProvider } from '@/components/providers/LogLevelProvider';
import NotifyProvider from '@/components/providers/NotifyProvider';
import { PatchProvider } from '@/components/providers/PatchProvider';
import { TabProvider } from '@/components/providers/TabProvider';

import { DynamicThemeProvider } from './DynamicThemeProvider';

import type { ReactNode } from 'react';

type Props = Readonly<{ children: ReactNode }>;

export const GlobalProvider = ({ children }: Props) => {
  return (
    <DynamicThemeProvider>
      <NotifyProvider />
      <LogLevelProvider>
        <TabProvider>
          <EditorModeProvider>
            <JsProvider>
              <CssProvider>
                <PatchProvider>{children}</PatchProvider>
              </CssProvider>
            </JsProvider>
          </EditorModeProvider>
        </TabProvider>
      </LogLevelProvider>
    </DynamicThemeProvider>
  );
};
