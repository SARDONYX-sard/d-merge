// Copyright (c) 2023 Luma <lumakernel@gmail.com>
// SPDX-License-Identifier: MIT or Apache-2.0

import type { ReactNode } from 'react';
import { useBackup } from '@/components/hooks/useBackup';
import { PageNavigation } from '@/components/organisms/PageNavigation';
import { GlobalProvider } from '@/components/providers';
import { LANG } from '@/lib/i18n';
import { LOG } from '@/services/api/log';
import { showWindow } from '@/services/api/window';
import '@/services/api/global_events';

LANG.init();
LOG.changeLevel(LOG.get());

type Props = Readonly<{
  children: ReactNode;
}>;

const ClientLayout = ({ children }: Props) => {
  showWindow();

  return (
    <GlobalProvider>
      <ClientLayoutProviderInner>{children}</ClientLayoutProviderInner>
      <PageNavigation />
    </GlobalProvider>
  );
};

const ClientLayoutProviderInner = ({ children }: Props) => {
  // NOTE: If you place useInjectJS here, it will only be executed once.
  // NOTE: The following hooks will not be able to retrieve the state unless they are within GlobalProvider and will result in an error.
  useBackup();

  return <>{children}</>;
};

export default ClientLayout;
