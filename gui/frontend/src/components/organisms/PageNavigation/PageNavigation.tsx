import Layers from '@mui/icons-material/Layers';
import SettingsIcon from '@mui/icons-material/Settings';
import TransformIcon from '@mui/icons-material/Transform';
import BottomNavigation from '@mui/material/BottomNavigation';
import BottomNavigationAction from '@mui/material/BottomNavigationAction';
import { usePathname, useRouter } from 'next/navigation';
import { useEffect, useState } from 'react';
import { z } from 'zod';

import { PUB_CACHE_OBJ } from '@/lib/storage/cacheKeys';
import { schemaStorage } from '@/lib/storage/schemaStorage';

/**
 * # NOTE
 * The order of the arrays must be in the order of the `BottomNavigationAction` declarations.
 *
 * Otherwise it will jump to the wrong place.
 */
const validPathNames = ['/convert', '/', '/settings'] as const;
const lastPathSchema = z.enum(validPathNames);

type LastPathName = (typeof validPathNames)[number];

const getPageIndex = (pageName: LastPathName): 0 | 2 | 1 => {
  switch (pageName) {
    case '/convert':
      return 0;
    case '/settings':
      return 2;
    default:
      return 1; // Default to patch page
  }
};
/**
 * This is a function that absorbs the difference between tauri and electron's `window.location.pathname`.
 *
 * For example, in the case of `/convert`
 * - tauri: ‘/convert/’
 * - electron: '[...]//app.asar/frontend/convert'
 */
const pathnameToLastPathName = (path: string): LastPathName => {
  if (path.endsWith('/convert/') || path.endsWith('/convert')) {
    return '/convert';
  }

  if (path.endsWith('/settings/') || path.endsWith('/settings')) {
    return '/settings';
  }

  return '/';
};

/** HACK: To prevents the conversion button from being hidden because the menu is fixed. */
const MenuPadding = () => <div style={{ height: '56px' }} />;

export function PageNavigation() {
  const router = useRouter();
  const pathname = usePathname();
  const [selectedPage, setSelectedPage] = useState(0);
  const [lastPath, setLastPath] = schemaStorage.use(PUB_CACHE_OBJ.lastPath, lastPathSchema);
  const lastPathName = pathnameToLastPathName(pathname);

  useEffect(() => {
    // Check if we've already redirected in this session
    const hasRedirected = sessionStorage.getItem('hasRedirected');

    if (lastPath && lastPath !== lastPathName && !hasRedirected) {
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
    const currentPage = getPageIndex(lastPathName);
    setSelectedPage(currentPage);
    setLastPath(lastPathName);
  }, [pathname, setLastPath]);

  const handleNavigationChange = (pageIdx: number) => {
    setSelectedPage(pageIdx);
    router.push(validPathNames[pageIdx]);
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
