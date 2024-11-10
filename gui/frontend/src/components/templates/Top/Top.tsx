'use client';
import { Box, type SxProps, type Theme } from '@mui/material';

import { useInjectJs } from '@/components/hooks/useInjectJs';
import { ModsGridWrapper } from '@/components/organisms/ModsGrid';
import { PatchProvider } from '@/components/organisms/ModsGrid/PatchProvider';

const sx: SxProps<Theme> = {
  display: 'grid',
  justifyContent: 'center',
  minHeight: 'calc(100vh - 56px)',
  width: '100%',
};

export const Top = () => {
  useInjectJs();

  return (
    <Box component='main' sx={sx}>
      <PatchProvider>
        <ModsGridWrapper />
      </PatchProvider>
    </Box>
  );
};
