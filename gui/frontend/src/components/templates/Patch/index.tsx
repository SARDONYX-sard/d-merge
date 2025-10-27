'use client'; // If this directive is not present on each page, a build error will occur.
import { Box, type SxProps, type Theme } from '@mui/material';

import { useInjectJs } from '@/components/hooks/useInjectJs';
import { PatchContainer } from '@/components/organisms/PatchContainer';

export const Patch = () => {
  useInjectJs();

  return (
    <Box component='main' sx={sx}>
      <PatchContainer />
    </Box>
  );
};

const sx: SxProps<Theme> = {
  display: 'grid',
  justifyContent: 'center',
  minHeight: 'calc(100vh - 56px)',
  width: '100%',
};
