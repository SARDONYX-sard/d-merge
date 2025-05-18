import { Box } from '@mui/material';
import { type MouseEventHandler, useState } from 'react';

import { useTimer } from '@/components/hooks/useTimer';
import { useTranslation } from '@/components/hooks/useTranslation';
import { InputField } from '@/components/molecules/InputField/InputField';
import { ConvertNav } from '@/components/organisms/ConvertNav';
import { ModsGrid } from '@/components/organisms/PatchContainer/ModsGrid';
import { usePatchContext } from '@/components/organisms/PatchContainer/PatchProvider';
import { usePatchInputs } from '@/components/organisms/PatchContainer/usePatchInputs';
import { NOTIFY } from '@/lib/notify';
import { patch } from '@/services/api/patch';

export const PatchContainer = () => {
  const { text: elapsedText, start: startTimer, stop: stopTimer } = useTimer();

  const { output, activateMods } = usePatchContext();
  const [loading, setLoading] = useState(false);
  const inputFieldsProps = usePatchInputs();
  const { t } = useTranslation();

  const handleClick: MouseEventHandler<HTMLButtonElement> = async (_e) => {
    setLoading(true);
    startTimer();

    try {
      await patch(output, activateMods);
      NOTIFY.success(`${t('patch-complete')} (${elapsedText})`);
    } catch (error) {
      NOTIFY.error(`Time: (${elapsedText})\n\n${error}`);
    } finally {
      stopTimer();
      setLoading(false);
    }
  };

  const loadingText = `${t('patching-btn')} (${elapsedText})`;

  return (
    <>
      <Box>
        {inputFieldsProps.map((inputProps) => {
          return <InputField key={inputProps.label} {...inputProps} />;
        })}
      </Box>
      <ModsGrid
        sx={{
          backgroundColor: '#160b0b60',
          marginTop: '10px',
          width: '95vw',
          maxHeight: '65vh',
        }}
      />
      <ConvertNav buttonText={t('patch-btn')} loading={loading} loadingText={loadingText} onClick={handleClick} />
    </>
  );
};
