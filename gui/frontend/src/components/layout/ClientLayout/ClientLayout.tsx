// Copyright (c) 2023 Luma <lumakernel@gmail.com>
// SPDX-License-Identifier: MIT or Apache-2.0
'use client';
import { CssBaseline } from '@mui/material';

import { Footer } from '@/components/organisms/Footer';
import { GlobalProvider } from '@/components/providers';
import { LANG } from '@/lib/i18n';
import { LOG } from '@/services/api/log';
import { showWindow } from '@/services/api/window';

import type { ReactNode } from 'react';

LANG.init();
LOG.changeLevel(LOG.get());

type Props = Readonly<{
  children: ReactNode;
}>;

export const ClientLayout = ({ children }: Props) => {
  showWindow();

  return (
    <GlobalProvider>
      <CssBaseline />
      {children}
      <Footer />
    </GlobalProvider>
  );
};
