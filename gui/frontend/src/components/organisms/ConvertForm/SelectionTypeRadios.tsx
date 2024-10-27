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
      <FormLabel component='legend' id={radioLabelId}>
        Selection Type
      </FormLabel>
      <RadioGroup
        aria-labelledby={radioLabelId}
        name='radio-buttons-group'
        onChange={handleSelectionTypeChange}
        row={true}
        value={selectionType}
      >
        <FormControlLabel control={<Radio />} label='Files' value='files' />
        <FormControlLabel control={<Radio />} label='Directories' value='dir' />
      </RadioGroup>
    </FormControl>
  );
};
