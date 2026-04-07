import { createRootRoute, Outlet } from '@tanstack/react-router';
import ClientLayout from '@/components/layout/ClientLayout/ClientLayout';

export const Route = createRootRoute({
  component: RootComponent,
});

function RootComponent() {
  return (
    <>
      <ClientLayout>
        <Outlet />
      </ClientLayout>
    </>
  );
}
