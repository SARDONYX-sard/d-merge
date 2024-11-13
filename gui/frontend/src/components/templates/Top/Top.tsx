'use client'; // If this directive is not present on each page, a build error will occur.
import { Box, type SxProps, type Theme } from '@mui/material';

import { useInjectJs } from '@/components/hooks/useInjectJs';
import { PatchContainer } from '@/components/organisms/PatchContainer';
import { PatchProvider } from '@/components/organisms/PatchContainer/PatchProvider';

const sx: SxProps<Theme> = {
  display: 'grid',
  justifyContent: 'center',
  minHeight: 'calc(100vh - 56px)',
  width: '100%',
};

export const Top = () => {
  useInjectJs();

  return (
    <Box component="main" sx={sx}>
      <PatchProvider>
        <PatchContainer />
      </PatchProvider>
    </Box>
  );
};
