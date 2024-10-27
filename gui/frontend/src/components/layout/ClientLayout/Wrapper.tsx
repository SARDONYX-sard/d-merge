// NOTE: From next15, { ssr: false } can only be called by the Client component, so a wrapper is provided in one step.
'use client';
import dynamic from 'next/dynamic';

import Loading from '@/components/templates/Loading';

import type { ReactNode } from 'react';

const ClientLayoutInner = dynamic(() => import('@/components/layout/ClientLayout'), {
  loading: () => <Loading />,
  ssr: false,
});

type Props = Readonly<{
  children: ReactNode;
}>;

export const ClientLayout = ({ children }: Props) => {
  return <ClientLayoutInner>{children}</ClientLayoutInner>;
};
