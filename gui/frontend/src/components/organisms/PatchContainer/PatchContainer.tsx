import { Box } from '@mui/material';
import { type MouseEventHandler, useState } from 'react';

import { InputField } from '@/components/molecules/InputField/InputField';
import { ConvertNav } from '@/components/organisms/ConvertNav';
import { ModsGrid } from '@/components/organisms/PatchContainer/ModsGrid';
import { usePatchContext } from '@/components/organisms/PatchContainer/PatchProvider';
import { usePatchInputs } from '@/components/organisms/PatchContainer/usePatchInputs';
import { NOTIFY } from '@/lib/notify';
import { patch } from '@/services/api/patch';

export const PatchContainer = () => {
  const { output, activateMods } = usePatchContext();
  const [loading, setLoading] = useState(false);
  const inputFieldsProps = usePatchInputs();

  const handleClick: MouseEventHandler<HTMLButtonElement> = async (_e) => {
    setLoading(true);
    try {
      const startMs = performance.now();

      await patch(output, activateMods);
      setLoading(false);

      const endMs = performance.now();
      const durationMs = endMs - startMs;

      const seconds = Math.floor(durationMs / 1000);
      const ms = Math.round(durationMs % 1000);

      NOTIFY.success(`Generation Complete! (${seconds}.${ms}s)`);
    } catch (error) {
      NOTIFY.error(`${error}`);
    }
  };

  return (
    <>
      <Box>
        {inputFieldsProps.map((inputProps) => {
          return <InputField key={inputProps.label} {...inputProps} />;
        })}
      </Box>
      <ModsGrid
        sx={{
          marginTop: '10px',
          width: '95vw',
          maxHeight: '65vh',
        }}
      />
      <ConvertNav loading={loading} onClick={handleClick} />
    </>
  );
};
