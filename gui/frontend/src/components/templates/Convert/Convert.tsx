'use client';
import { Box, Grid2, type SxProps, type Theme } from '@mui/material';
import { type MouseEventHandler, useState } from 'react';

import { useInjectJs } from '@/components/hooks/useInjectJs';
import { ConvertForm } from '@/components/organisms/ConvertForm';
import { ConvertProvider, useConvertContext } from '@/components/organisms/ConvertForm/ConvertProvider';
import { ConvertNav } from '@/components/organisms/ConvertNav';
import { NOTIFY } from '@/lib/notify';
import { convert } from '@/services/api/serde_hkx';

const sx: SxProps<Theme> = {
  display: 'grid',
  placeContent: 'center',
  minHeight: 'calc(100vh - 56px)',
  width: '100%',
};

export const Convert = () => {
  useInjectJs();

  return (
    <Box component='main' sx={sx}>
      <ConvertProvider>
        <ConvertInner />
      </ConvertProvider>
    </Box>
  );
};

const ConvertInner = () => {
  const [loading, setLoading] = useState(false);
  const { input, output, fmt } = useConvertContext();

  const handleClick: MouseEventHandler<HTMLButtonElement> = async (_e) => {
    setLoading(true);
    await NOTIFY.asyncTry(async () => await convert(input, output, fmt));
    setLoading(false);
    NOTIFY.success(`Converted ${input}\n -> ${output}`);
  };

  return (
    <>
      <Grid2 sx={{ width: '90vw' }}>
        <ConvertForm />
      </Grid2>
      <ConvertNav loading={loading} onClick={handleClick} />
    </>
  );
};
