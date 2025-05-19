import { Box, Typography } from '@mui/material';
import { useState } from 'react';

import { useTimer } from '@/components/hooks/useTimer';
import { useTranslation } from '@/components/hooks/useTranslation';
import { InputField } from '@/components/molecules/InputField/InputField';
import { ConvertNav } from '@/components/organisms/ConvertNav';
import { ModsGrid } from '@/components/organisms/PatchContainer/ModsGrid';
import { usePatchHandler } from '@/components/organisms/PatchContainer/usePatchHandler';
import { usePatchInputs } from '@/components/organisms/PatchContainer/usePatchInputs';
import { NOTIFY } from '@/lib/notify';

export const PatchContainer = () => {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(false);
  const { text: elapsedText, start, stop } = useTimer();
  const [status, setStatus] = useState<string | null>(null);

  const [statusText, setStatusText] = useState('');

  const inputFieldsProps = usePatchInputs();

  const { handleClick } = usePatchHandler({
    setLoading,
    start,
    onStatus: (s, unlisten) => {
      setStatus(s);

      switch (s) {
        case 'ReadingTemplatesAndPatches': {
          setStatusText(t('patch.patch_reading_message'));
          break;
        }
        case 'ApplyingPatches': {
          setStatusText(t('patch.patch_applying_message'));
          break;
        }
        case 'Done': {
          setStatusText(t('patch.patch_complete_message'));
          setLoading(false);
          unlisten?.();
          break;
        }
        default:
          break;
      }
    },
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
        <Typography sx={{ mt: 1, mb: 1 }} variant='body2'>
          Status: {statusText}
        </Typography>
      )}

      <ConvertNav buttonText={t('patch.button')} loading={loading} loadingText={loadingText} onClick={handleClick} />
    </>
  );
};
