'use client';
import { Box, type SxProps, type Theme } from '@mui/material';
import { type MouseEventHandler, useState } from 'react';

import { useInjectJs } from '@/components/hooks/useInjectJs';
import { useModsInfo } from '@/components/hooks/useModsInfo';
import { ConvertNav } from '@/components/organisms/ConvertNav';
import { ModsGrid } from '@/components/organisms/ModsGrid';
import { NOTIFY } from '@/lib/notify';
import { patch } from '@/services/api/patch';

const sx: SxProps<Theme> = {
  display: 'grid',
  placeContent: 'center',
  minHeight: 'calc(100vh - 56px)',
  width: '100%',
};

export const Top = () => {
  useInjectJs();
  const [loading, setLoading] = useState(false);
  const modsInfoProps = useModsInfo();
  const { selectionModel } = modsInfoProps;

  const handleClick: MouseEventHandler<HTMLButtonElement> = async (_e) => {
    setLoading(true);
    await NOTIFY.asyncTry(async () => await patch(selectionModel));
    await new Promise((r) => setTimeout(r, 1000));
    setLoading(false);
  };

  return (
    <Box component='main' sx={sx}>
      <ModsGrid
        sx={{
          marginTop: '10px',
          width: '95vw',
          maxHeight: '80vh',
        }}
        {...modsInfoProps}
      />
      <ConvertNav loading={loading} onClick={handleClick} />
    </Box>
  );
};
