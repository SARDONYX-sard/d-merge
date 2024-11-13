import Layers from '@mui/icons-material/Layers';
import SettingsIcon from '@mui/icons-material/Settings';
import TransformIcon from '@mui/icons-material/Transform';
import BottomNavigation from '@mui/material/BottomNavigation';
import BottomNavigationAction from '@mui/material/BottomNavigationAction';
import { usePathname, useRouter } from 'next/navigation';
import { useEffect, useState } from 'react';
import { z } from 'zod';

import { PUB_CACHE_OBJ } from '@/lib/storage/cacheKeys';

import { schemaStorage } from '../../../lib/storage/schemaStorage';

const validLastPath = ['settings', 'convert', '/'] as const;
const lastPathSchema = z.enum(validLastPath);
const getPageIndex = (path: string) => {
  switch (path) {
    case '/convert':
      return 0;
    case '/':
      return 1;
    case '/settings':
      return 2;
    default:
      return 0;
  }
};

/** HACK: To prevents the conversion button from being hidden because the menu is fixed. */
const MenuPadding = () => <div style={{ height: '56px' }} />;

export function PageNavigation() {
  const router = useRouter();
  const pathname = usePathname();
  const [selectedPage, setSelectedPage] = useState(0);
  const [lastPath, setLastPath] = schemaStorage.use(PUB_CACHE_OBJ.lastPath, lastPathSchema);

  useEffect(() => {
    // Check if we've already redirected in this session
    const hasRedirected = sessionStorage.getItem('hasRedirected');

    if (lastPath && lastPath !== pathname && !hasRedirected) {
      sessionStorage.setItem('hasRedirected', 'true');
      // Since `/` is the initial coming path, there is no need to jump.
      // If you jump, you will have to jump twice when `/` is the LAST PATH.
      if (lastPath === '/') {
        return;
      }
      router.push(lastPath);
    }
  }, [lastPath, pathname, router]);

  useEffect(() => {
    const currentPage = getPageIndex(pathname);
    setSelectedPage(currentPage);

    const result = lastPathSchema.safeParse(pathname);
    if (result.success) {
      setLastPath(result.data);
    }
  }, [pathname, setLastPath]);

  const handleNavigationChange = (pageIdx: number) => {
    setSelectedPage(pageIdx);
    const paths = ['/convert', '/', '/settings'];
    router.push(paths[pageIdx]);
  };

  return (
    <>
      <MenuPadding />
      <BottomNavigation
        onChange={(_event, newPageIdx: number) => handleNavigationChange(newPageIdx)}
        showLabels={true}
        sx={{
          position: 'fixed',
          bottom: 0,
          width: '100%',
          zIndex: '100',
          '.Mui-selected': {
            color: '#99e4ee',
          },
        }}
        value={selectedPage}
      >
        <BottomNavigationAction icon={<TransformIcon />} label='Convert' />
        <BottomNavigationAction icon={<Layers />} label='Patch' />
        <BottomNavigationAction icon={<SettingsIcon />} label='Settings' />
      </BottomNavigation>
    </>
  );
}
