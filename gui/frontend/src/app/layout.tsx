import dynamic from 'next/dynamic';

export { metadata } from '@/components/meta/meta';
import { inter } from '@/components/meta/font';
import Loading from '@/components/templates/Loading';

import type { ReactNode } from 'react';

import '@/app/globals.css';

const ClientLayout = dynamic(() => import('@/components/layout/ClientLayout'), {
  loading: () => <Loading />,
  ssr: false,
});

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
