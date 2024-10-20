'use client';

import { Box, Link } from '@mui/material';

import type { ComponentPropsWithoutRef } from 'react';

type Props = {
  version: string;
} & ComponentPropsWithoutRef<typeof Link>;
export const Help = ({ version, ...props }: Props) => {
  return (
    <Box
      sx={{
        display: 'flex',
        alignItems: 'center',
        flexDirection: 'column',
        height: '100%',
        justifyContent: 'space-evenly',
      }}
    >
      <div>Version: {version}</div>
      <div>
        Source:{' '}
        <Link rel='noopener' sx={{ fontSize: 'large' }} target='_blank' {...props}>
          GitHub
        </Link>
      </div>
    </Box>
  );
};
