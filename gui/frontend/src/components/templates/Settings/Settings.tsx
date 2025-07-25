'use client'; // If this directive is not present on each page, a build error will occur.
import { Box, type SxProps, type Theme } from '@mui/material';
import Grid from '@mui/material/Grid';
import type { MouseEventHandler } from 'react';
import packageJson from '@/../../package.json';
import { Help } from '@/components/atoms/Help';
import { useInjectJs } from '@/components/hooks/useInjectJs';
import { CodeEditorTab } from '@/components/organisms/CodeEditorTab';
import { Tabs } from '@/components/organisms/Tabs';
import { useTabContext } from '@/components/providers/TabProvider';
import { openUrl } from '@/services/api/shell';

const sx: SxProps<Theme> = {
  alignItems: 'center',
  display: 'flex',
  flexDirection: 'column',
  justifyContent: 'center',
  minHeight: 'calc(100vh - 56px)',
  width: '100%',
};

export const Settings = () => {
  const { tabPos } = useTabContext();
  useInjectJs();

  return (
    <Box component='main' sx={sx}>
      {tabPos === 'top' ? (
        <>
          <TabsMenu />
          <CodeEditorTab />
        </>
      ) : (
        <>
          <CodeEditorTab />
          <TabsMenu />
        </>
      )}
    </Box>
  );
};

const TabsMenu = () => {
  const handleHelpClick: MouseEventHandler<HTMLButtonElement> = (_event) => {
    openUrl(packageJson.homepage); // jump by backend api
  };

  return (
    <Grid container={true} sx={{ width: '95%' }}>
      <Grid size={8} sx={{ overflowX: 'auto' }}>
        <Tabs />
      </Grid>
      <Grid size={4} sx={{ overflowX: 'auto' }}>
        <Help onClick={handleHelpClick} version={packageJson.version} />
      </Grid>
    </Grid>
  );
};
