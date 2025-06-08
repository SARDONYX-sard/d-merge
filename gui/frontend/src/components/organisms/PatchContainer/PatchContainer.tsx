import { Box, Typography } from '@mui/material';
import { useState } from 'react';

import { useTimer } from '@/components/hooks/useTimer';
import { useTranslation } from '@/components/hooks/useTranslation';
import { InputField } from '@/components/molecules/InputField/InputField';
import { BottomActionBar } from '@/components/organisms/BottomActionBar';
import { usePatchHandler } from '@/components/organisms/PatchContainer/hooks/usePatchHandler';
import { usePatchInputs } from '@/components/organisms/PatchContainer/hooks/usePatchInputs';
import { ModsGrid } from '@/components/organisms/PatchContainer/ModsGrid';
import { NOTIFY } from '@/lib/notify';

import { useBackup } from './hooks/useBackup';
import { usePatchStatus } from './hooks/usePatchStatus';
import { PatchOptionsDialog } from './PatchOptionsButtonDialog';

export const PatchContainer = () => {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(false);
  const { text: elapsedText, start, stop } = useTimer();
  useBackup();

  const { status, statusText, handleStatus } = usePatchStatus(stop, setLoading);
  const inputFieldsProps = usePatchInputs();

  const { handleClick } = usePatchHandler({
    setLoading,
    start,
    onStatus: handleStatus,
    onError: (err) => {
      setLoading(false);
      NOTIFY.error(`${err} (${stop()})`);
    },
  });

  const loadingText = `${t('patch.patching_button')} (${elapsedText})`;

  return (
    <>
      <Box>
        {inputFieldsProps.map((inputProps) => (
          <InputField key={inputProps.label} {...inputProps} />
        ))}
      </Box>

      <ModsGrid
        sx={{
          backgroundColor: '#160b0b60',
          marginTop: '10px',
          width: '95vw',
          maxHeight: '65vh',
        }}
      />
      {status && (
        <Typography sx={{ mt: 1, mb: 0, textAlign: 'right' }} variant='body2'>
          Status: {statusText}
        </Typography>
      )}

      <BottomActionBar buttonText={t('patch.button')} loading={loading} loadingText={loadingText} onClick={handleClick}>
        <PatchOptionsDialog />
      </BottomActionBar>
    </>
  );
};
