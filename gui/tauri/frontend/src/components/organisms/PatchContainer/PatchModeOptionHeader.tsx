import {
  Checkbox,
  FormControl,
  FormControlLabel,
  FormGroup,
  MenuItem,
  Radio,
  RadioGroup,
  Select,
  SelectChangeEvent,
} from '@mui/material';
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

  const handleParserModeChange = useCallback(
    (event: SelectChangeEvent) => {
      const value = event.target.value;

      if (value === 'strict' || value === 'lenient') {
        setPatchOptions((prev) => ({
          ...prev,
          parserMode: value,
        }));
      }
    },
    [setPatchOptions],
  );

  const handleSkeletonArmFixChange = useCallback(
    (_: React.ChangeEvent<HTMLInputElement>, checked: boolean) => {
      setPatchOptions((prev) => ({ ...prev, skeletonArmFix: checked }));
    },
    [setPatchOptions],
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
          <FormControlLabel value='vfs' control={<Radio />} label='VFS' />
          <FormControlLabel value='manual' control={<Radio />} label='Manual' />
        </RadioGroup>

        <Select
          size='small'
          aria-label='Parser mode'
          value={patchOptions.parserMode ?? 'strict'}
          onChange={handleParserModeChange}
        >
          <MenuItem value='strict'>Strict</MenuItem>
          <MenuItem value='lenient'>Lenient</MenuItem>
        </Select>

        <FormControlLabel
          label='Skeleton Arm Fix'
          control={<Checkbox checked={patchOptions.skeletonArmFix} onChange={handleSkeletonArmFixChange} />}
        />

        <FormControlLabel
          label='FNIS.esp'
          control={<Checkbox checked={patchOptions.generateFnisEsp} onChange={handleGenerateFnisEsp} />}
        />
      </FormGroup>
    </FormControl>
  );
};
