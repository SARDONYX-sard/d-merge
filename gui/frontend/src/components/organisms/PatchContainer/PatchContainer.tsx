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
    const startMs = performance.now();
    startTimer();

    try {
      await patch(output, activateMods);
      stopTimer();
      NOTIFY.success(`${t('patch-complete')} (${elapsedToText(startMs)})`);
    } catch (error) {
      stopTimer();
      NOTIFY.error(`Time: (${elapsedToText(startMs)})\n\n${error}`);
    } finally {
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

function elapsedToText(startMs: number) {
  const endMs = performance.now();
  const elapsed = endMs - startMs;
  const seconds = Math.floor(elapsed / 1000);
  const ms = Math.floor(elapsed % 1000);
  return `${seconds}.${ms.toString().padStart(3, '0')}s`;
}
