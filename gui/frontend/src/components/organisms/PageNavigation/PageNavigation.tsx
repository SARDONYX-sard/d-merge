'use client';

import HomeIcon from '@mui/icons-material/Home';
import LayersIcon from '@mui/icons-material/Layers';
import NotesIcon from '@mui/icons-material/Notes';
import SettingsIcon from '@mui/icons-material/Settings';
import TransformIcon from '@mui/icons-material/Transform';
import BottomNavigation from '@mui/material/BottomNavigation';
import BottomNavigationAction from '@mui/material/BottomNavigationAction';
import type { SxProps, Theme } from '@mui/material/styles';
import type { JSX } from 'react';
import { usePageRedirect } from './usePageRedirect';

export const validPaths = ['/convert', '/patch', '/', '/hkanno', '/settings'] as const;

const pathIcons: Record<(typeof validPaths)[number], JSX.Element> = {
  '/': <HomeIcon />,
  '/convert': <TransformIcon />,
  '/patch': <LayersIcon />,
  '/hkanno': <NotesIcon />,
  '/settings': <SettingsIcon />,
};

const pathLabels: Record<(typeof validPaths)[number], string> = {
  '/': 'Home',
  '/convert': 'Convert',
  '/patch': 'Patch',
  '/hkanno': 'HKAnno',
  '/settings': 'Settings',
};

export const PageNavigation = () => {
  const { selectedIndex, navigateTo } = usePageRedirect(validPaths);
  // height: Use z-index to occupy the space it occupies so it doesn't appear on top of other pages.
  return (
    <>
      <div style={{ height: '56px' }} />
      <BottomNavigation showLabels sx={navStyles} onChange={(_e, idx) => navigateTo(idx)} value={selectedIndex}>
        {validPaths.map((path) => (
          <BottomNavigationAction key={path} icon={pathIcons[path]} label={pathLabels[path]} />
        ))}
      </BottomNavigation>
    </>
  );
};

const navStyles = (theme: Theme) =>
  ({
    position: 'fixed',
    bottom: 0,
    left: '50%',
    transform: 'translateX(-50%)',
    width: '100%',
    zIndex: '100',
    backdropFilter: 'blur(4px)',
    bgcolor: theme.palette.mode === 'dark' ? 'rgba(20, 20, 20, 0.8)' : 'rgba(255, 255, 255, 0.7)',
    boxShadow: theme.palette.mode === 'dark' ? '0 2px 8px rgba(0,0,0,0.4)' : '0 2px 8px rgba(0,0,0,0.1)',
    transition: 'background-color 0.3s, box-shadow 0.3s, transform 0.3s',
    '& .MuiBottomNavigationAction-root': {
      transition: 'transform 0.2s, color 0.2s',
      '&:hover': {
        transform: 'translateY(-2px) scale(1.05)',
        color: 'var(--mui-palette-primary-main)',
      },
    },
    '.Mui-selected': {
      color: 'var(--mui-palette-primary-main)',
    },
  }) as const satisfies SxProps<Theme>;
