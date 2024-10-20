'use client';
import { Box, type SxProps, type Theme } from '@mui/material';
import Grid from '@mui/material/Grid2';

import { Help } from '@/components/atoms/Help';
import { useInjectJs } from '@/components/hooks/useInjectJs';
import { CodeEditor } from '@/components/organisms/CodeEditor';
import { Tabs } from '@/components/organisms/Tabs';
import { start } from '@/services/api/shell';

import packageJson from '@/../../package.json';

import type { MouseEventHandler } from 'react';

const sx: SxProps<Theme> = {
  alignItems: 'center',
  display: 'flex',
  flexDirection: 'column',
  justifyContent: 'center',
  minHeight: 'calc(100vh - 56px)',
  width: '100%',
};

export const Settings = () => {
  useInjectJs();

  const handleHelpClick: MouseEventHandler<HTMLAnchorElement> = (event) => {
    event.preventDefault(); // Avoid to jump by browser
    start(packageJson.homepage); // jump by backend api
  };

  return (
    <Box component='main' sx={sx}>
      <CodeEditor />

      <Grid container={true} sx={{ width: '95%' }}>
        <Grid size={8} sx={{ overflowX: 'auto' }}>
          <Tabs />
        </Grid>
        <Grid size={4} sx={{ overflowX: 'auto' }}>
          <Help href={packageJson.homepage} onClick={handleHelpClick} version={packageJson.version} />
        </Grid>
      </Grid>
    </Box>
  );
};
