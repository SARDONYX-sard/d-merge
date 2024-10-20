'use client';

import Layers from '@mui/icons-material/Layers';
import SettingsIcon from '@mui/icons-material/Settings';
import TransformIcon from '@mui/icons-material/Transform';
import BottomNavigation from '@mui/material/BottomNavigation';
import BottomNavigationAction from '@mui/material/BottomNavigationAction';
import { usePathname, useRouter } from 'next/navigation';
import { useEffect, useState } from 'react';

/** HACK: To prevents the conversion button from being hidden because the menu is fixed. */
const MenuPadding = () => <div style={{ height: '56px' }} />;

export function Footer() {
  const router = useRouter();
  const pathname = usePathname();
  const [value, setValue] = useState(1);

  useEffect(() => {
    if (pathname === '/convert') {
      setValue(0);
    } else if (pathname === '/') {
      setValue(1);
    } else if (pathname === '/settings') {
      setValue(2);
    }
  }, [pathname]);

  return (
    <>
      <MenuPadding />
      <BottomNavigation
        onChange={(_event, newValue) => {
          setValue(newValue);
        }}
        showLabels={true}
        sx={{
          position: 'fixed',
          bottom: 0,
          width: '100%',
          zIndex: '100', // Because Ace-editor uses z-index and without it, it would be covered.
          '.Mui-selected': {
            color: '#99e4ee',
          },
        }}
        value={value}
      >
        <BottomNavigationAction icon={<TransformIcon />} label='Convert' onClick={() => router.push('/convert')} />
        <BottomNavigationAction icon={<Layers />} label='Patch' onClick={() => router.push('/')} />
        <BottomNavigationAction icon={<SettingsIcon />} label='Settings' onClick={() => router.push('/settings')} />
      </BottomNavigation>
    </>
  );
}
