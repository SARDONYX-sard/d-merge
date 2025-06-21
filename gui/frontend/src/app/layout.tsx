export { metadata } from '@/components/meta/meta';

import type { ReactNode } from 'react';
import ClientLayout from '@/components/layout/ClientLayout';
import { inter } from '@/components/meta/font';

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
