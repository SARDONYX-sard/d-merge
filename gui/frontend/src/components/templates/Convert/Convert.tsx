'use client'; // If this directive is not present on each page, a build error will occur.
import { Box, Grid, type SxProps, type Theme } from '@mui/material';
import { listen } from '@tauri-apps/api/event';
import { type MouseEventHandler, useState } from 'react';

import { useInjectJs } from '@/components/hooks/useInjectJs';
import { BottomActionBar } from '@/components/organisms/BottomActionBar';
import { ConvertForm } from '@/components/organisms/ConvertForm';
import {
  ConvertProvider,
  type ConvertStatusPayload,
  useConvertContext,
} from '@/components/organisms/ConvertForm/ConvertProvider';
import { getAllLeafItemIds } from '@/components/organisms/ConvertForm/PathTreeSelector';
import { NOTIFY } from '@/lib/notify';
import { convert } from '@/services/api/serde_hkx';

export const Convert = () => {
  useInjectJs();

  return (
    <ConvertProvider>
      <ProviderInner />
    </ConvertProvider>
  );
};

const sx: SxProps<Theme> = {
  display: 'grid',
  paddingTop: '5%',
  alignItems: 'top',
  justifyItems: 'center',
  minHeight: 'calc(100vh - 56px)',
  width: '100%',
};

const ProviderInner = () => {
  const { loading, handleClick } = useConvertExec();

  return (
    <Box component='main' sx={sx}>
      <Grid sx={{ width: '90vw' }}>
        <ConvertForm />
      </Grid>
      <BottomActionBar loading={loading} onClick={handleClick} />
    </Box>
  );
};

const useConvertExec = () => {
  const [loading, setLoading] = useState(false);
  const { selectedFiles, selectedDirs, selectedTree, output, fmt, selectionType, setConvertStatuses } =
    useConvertContext();

  const [inputs, roots] = (() => {
    switch (selectionType) {
      case 'dir':
        return [selectedDirs, undefined];
      case 'files':
        return [selectedFiles, undefined];
      case 'tree': {
        const { selectedItems, tree, roots } = selectedTree;
        return [getAllLeafItemIds(selectedItems, tree), roots];
      }
      default:
        return [[], undefined];
    }
  })();

  const handleClick: MouseEventHandler<HTMLButtonElement> = async (_e) => {
    const eventHandler = (event: { payload: ConvertStatusPayload }) => {
      setConvertStatuses((prev) => {
        const { pathId, status } = event.payload;
        prev.set(pathId, status);
        // NOTE: As with Object, if the same reference is returned,
        // the value is not recognized as updated! So we need to call a new constructor sequentially.
        return new Map(prev);
      });
    };

    const unlisten = await listen('d_merge://progress/convert', eventHandler);
    try {
      setLoading(true);
      await convert(inputs, output, fmt, roots);
    } catch (e) {
      NOTIFY.error(`${e}`);
    } finally {
      unlisten();
      setLoading(false);
    }
  };

  return {
    loading,
    handleClick,
  };
};
