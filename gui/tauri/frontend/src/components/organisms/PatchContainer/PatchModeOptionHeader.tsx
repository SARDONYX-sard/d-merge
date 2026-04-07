import { FormControl, FormControlLabel, FormGroup, Radio, RadioGroup } from '@mui/material';
import { Checkbox } from '@mui/material';
import { useCallback } from 'react';
import { usePatchContext } from '@/components/providers/PatchProvider';

export const PatchModeOptionHeader = () => {
  const { isVfsMode, setIsVfsMode, patchOptions, setPatchOptions } = usePatchContext();

  const handleChange = useCallback(
    (_: React.ChangeEvent<HTMLInputElement>, value: string) => {
      setIsVfsMode(value === 'vfs');
    },
    [setIsVfsMode],
  );

  const handleGenerateFnisEsp = useCallback(
    (_: React.ChangeEvent<HTMLInputElement>, checked: boolean) => {
      setPatchOptions((prev) => ({ ...prev, generateFnisEsp: checked }));
    },
    [setPatchOptions],
  );

  return (
    <FormControl>
      <FormGroup row>
        <RadioGroup row value={isVfsMode ? 'vfs' : 'manual'} onChange={handleChange}>
          <FormControlLabel value='vfs' control={<Radio />} label={'VFS'} />
          <FormControlLabel value='manual' control={<Radio />} label={'Manual'} />
        </RadioGroup>

        <FormControlLabel
          label='FNIS.esp'
          control={<Checkbox checked={patchOptions.generateFnisEsp} onChange={handleGenerateFnisEsp} />}
        />
      </FormGroup>
    </FormControl>
  );
};
