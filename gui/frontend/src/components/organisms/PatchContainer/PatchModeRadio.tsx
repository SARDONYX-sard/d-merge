import { FormControl, FormControlLabel, Radio, RadioGroup } from '@mui/material';
import { useCallback } from 'react';

import { usePatchContext } from '@/components/providers/PatchProvider';

export const PatchModeRadio = () => {
  const { isVfsMode, setIsVfsMode } = usePatchContext();

  const handleChange = useCallback(
    (_: React.ChangeEvent<HTMLInputElement>, value: string) => {
      setIsVfsMode(value === 'vfs');
    },
    [setIsVfsMode],
  );

  return (
    <FormControl>
      <RadioGroup row value={isVfsMode ? 'vfs' : 'manual'} onChange={handleChange}>
        <FormControlLabel value='vfs' control={<Radio />} label={'VFS'} />
        <FormControlLabel value='manual' control={<Radio />} label={'Manual'} />
      </RadioGroup>
    </FormControl>
  );
};
