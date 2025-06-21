import { FormControl, FormControlLabel, FormLabel, Radio, RadioGroup, Tooltip } from '@mui/material';
import { type ComponentPropsWithRef, useId } from 'react';

import { useTranslation } from '@/components/hooks/useTranslation';

import { useConvertContext } from './ConvertProvider';
import { selectionTypeSchema } from './schemas/selection_type';

export const SelectionTypeRadios = () => {
  const { selectionType, setSelectionType, setConvertStatuses } = useConvertContext();
  const { t } = useTranslation();
  const radioLabelId = useId();

  const handleSelectionTypeChange: ComponentPropsWithRef<'input'>['onChange'] = (event) => {
    setSelectionType(selectionTypeSchema.parse(event.target.value));
    setConvertStatuses(new Map()); // Clear to prevent mixing of file index and dir index status.
  };

  const options = [
    {
      label: t('convert.selection_type.directories'),
      value: 'dir',
      tooltip: t('convert.selection_type.directories_tooltip'),
    },
    { label: t('convert.selection_type.tree'), value: 'tree', tooltip: t('convert.selection_type.tree_tooltip') },
    { label: t('convert.selection_type.files'), value: 'files', tooltip: t('convert.selection_type.files_tooltip') },
  ];

  return (
    <FormControl>
      <FormLabel component='legend' id={radioLabelId}>
        {t('convert.selection_type.label')}
      </FormLabel>
      <RadioGroup
        aria-labelledby={radioLabelId}
        name='radio-buttons-group'
        onChange={handleSelectionTypeChange}
        row={true}
        value={selectionType}
      >
        {options.map((option) => (
          <Tooltip key={option.value} placement='top' title={option.tooltip}>
            <FormControlLabel control={<Radio />} label={option.label} value={option.value} />
          </Tooltip>
        ))}
      </RadioGroup>
    </FormControl>
  );
};
