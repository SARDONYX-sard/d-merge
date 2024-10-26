import { FormControl, FormControlLabel, FormLabel, Radio, RadioGroup } from '@mui/material';
import { type ComponentPropsWithRef, useId } from 'react';

import { normalize, useConvertContext } from './ConvertProvider';

export const SelectionTypeRadios = () => {
  const { selectionType, setSelectionType, setConvertStatuses } = useConvertContext();
  const radioLabelId = useId();

  const handleSelectionTypeChange: ComponentPropsWithRef<'input'>['onChange'] = (event) => {
    setSelectionType(normalize(event.target.value));
    setConvertStatuses(new Map()); // Clear to prevent mixing of file index and dir index status.
  };

  return (
    <FormControl>
      <FormLabel id={radioLabelId} component='legend'>
        Selection Type
      </FormLabel>
      <RadioGroup
        aria-labelledby={radioLabelId}
        name='radio-buttons-group'
        onChange={handleSelectionTypeChange}
        row={true}
        value={selectionType}
      >
        <FormControlLabel value='files' control={<Radio />} label='Files' />
        <FormControlLabel value='dir' control={<Radio />} label='Directories' />
      </RadioGroup>
    </FormControl>
  );
};
