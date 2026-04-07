'use client';

import Box from '@mui/material/Box';
import React from 'react';
import { useInjectJs } from '@/components/hooks/useInjectJs';
import { HkannoTabEditor } from '@/components/organisms/HkAnnoEditor/HkAnnoTabEditor';

export const HkannoEditorPage: React.FC = () => {
  useInjectJs();

  return (
    <Box
      component='main'
      sx={{
        display: 'flex',
        flexDirection: 'column',
        minHeight: 'calc(100vh - 56px)',
        position: 'relative',
      }}
    >
      <HkannoTabEditor />
    </Box>
  );
};
