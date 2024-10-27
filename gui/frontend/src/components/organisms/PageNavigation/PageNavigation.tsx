'use client';

import Layers from '@mui/icons-material/Layers';
import SettingsIcon from '@mui/icons-material/Settings';
import TransformIcon from '@mui/icons-material/Transform';
import BottomNavigation from '@mui/material/BottomNavigation';
import BottomNavigationAction from '@mui/material/BottomNavigationAction';
import { usePathname, useRouter } from 'next/navigation';
import { useEffect, useState } from 'react';

import { STORAGE } from '@/lib/storage';
import { PUB_CACHE_OBJ } from '@/lib/storage/cacheKeys';

/** HACK: To prevents the conversion button from being hidden because the menu is fixed. */
const MenuPadding = () => <div style={{ height: '56px' }} />;

export function PageNavigation() {
  const router = useRouter();
  const pathname = usePathname();
  const [selectedPage, setSelectedPage] = useState(0);

  useEffect(() => {
    // Check if we've already redirected in this session
    const hasRedirected = sessionStorage.getItem('hasRedirected');
    const lastPath = STORAGE.get(PUB_CACHE_OBJ.lastPath);

    // If there's a lastPath, and we haven't redirected yet, navigate to the last path
    if (lastPath && lastPath !== pathname && !hasRedirected) {
      sessionStorage.setItem('hasRedirected', 'true');
      router.push(lastPath);
    }
  }, [pathname, router]);

  useEffect(() => {
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

    const currentPage = getPageIndex(pathname);
    setSelectedPage(currentPage);

    STORAGE.set(PUB_CACHE_OBJ.lastPath, pathname); // Save current path as the last visited path in localStorage
  }, [pathname]);

  const handleNavigationChange = (newValue: number) => {
    setSelectedPage(newValue);
    const paths = ['/convert', '/', '/settings'];
    router.push(paths[newValue]);
  };

  return (
    <>
      <MenuPadding />
      <BottomNavigation
        onChange={(_event, newValue) => handleNavigationChange(newValue)}
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
