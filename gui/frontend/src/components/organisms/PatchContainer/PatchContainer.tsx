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
    await NOTIFY.asyncTry(async () => await patch(output, activateMods));
    await new Promise((r) => setTimeout(r, 1000));
    setLoading(false);
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
