import { ClientLayout } from '@/components/layout/ClientLayout/Wrapper';
import { inter } from '@/components/meta/font';

import type { ReactNode } from 'react';

import '@/app/globals.css';

type Props = Readonly<{
  children: ReactNode;
}>;
export default function RootLayout({ children }: Props) {
  return (
    <html lang='en'>
      <body className={inter.className}>
        <ClientLayout>{children}</ClientLayout>
      </body>
    </html>
  );
}
