import { useCallback } from 'react';

import { SelectWithLabel } from '@/components/molecules/SelectWithLabel';

import { useConvertContext } from './ConvertProvider';

import type { SelectChangeEvent } from '@mui/material';

export const DirModeList = () => {
  const { pathMode, setPathMode } = useConvertContext();
  const handleOnChange = useCallback(
    ({ target }: SelectChangeEvent) => {
      if (target.value === 'path') {
        setPathMode('path');
      } else {
        setPathMode('dir');
      }
    },
    [setPathMode],
  );

  const menuItems = [
    { value: 'path', label: 'Path' },
    { value: 'dir', label: 'Dir' },
  ] as const;

  return <SelectWithLabel label={'Path Mode'} menuItems={menuItems} onChange={handleOnChange} value={pathMode} />;
};
