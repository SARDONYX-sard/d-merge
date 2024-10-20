import { useCallback } from 'react';

import { SelectWithLabel } from '@/components/molecules/SelectWithLabel';

import { useConvertContext } from './ConvertProvider';

import type { SelectChangeEvent } from '@mui/material';

export const OutFormatList = () => {
  const { fmt, setFmt } = useConvertContext();

  const handleOnChange = useCallback(
    ({ target }: SelectChangeEvent) => {
      switch (target.value) {
        case 'amd64':
        case 'win32':
        case 'xml':
          setFmt(target.value);
          break;
        default:
          setFmt('amd64');
          break;
      }
    },
    [setFmt],
  );

  const menuItems = [
    { value: 'amd64', label: 'Amd64' },
    { value: 'win32', label: 'Win32' },
    { value: 'xml', label: 'XML' },
  ] as const;

  return <SelectWithLabel label={'Output Format'} menuItems={menuItems} onChange={handleOnChange} value={fmt} />;
};
