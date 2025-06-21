import type { SelectChangeEvent } from '@mui/material';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from '@/components/hooks/useTranslation';
import { SelectWithLabel } from '@/components/molecules/SelectWithLabel';
import { NOTIFY } from '@/lib/notify';
import { isSupportedExtraFmt } from '@/services/api/serde_hkx';
import { useConvertContext } from './ConvertProvider';

export const OutFormatList = () => {
  const { fmt, setFmt } = useConvertContext();
  const [isSupportedExtra, setIsSupportedExtra] = useState(false);
  const { t } = useTranslation();

  useEffect(() => {
    NOTIFY.asyncTry(async () => {
      setIsSupportedExtra(await isSupportedExtraFmt());
    });
  }, []);

  const handleOnChange = useCallback(
    ({ target }: SelectChangeEvent) => {
      switch (target.value) {
        case 'amd64':
        case 'win32':
        case 'xml':
        case 'json':
        case 'yaml':
          setFmt(target.value);
          break;
        default:
          setFmt('amd64');
          break;
      }
    },
    [setFmt],
  );

  const extra = isSupportedExtra
    ? ([
        { value: 'json', label: 'Json' },
        { value: 'yaml', label: 'Yaml' },
      ] as const)
    : ([] as const);

  const menuItems = [
    { value: 'amd64', label: 'Amd64' },
    { value: 'win32', label: 'Win32' },
    { value: 'xml', label: 'XML' },
    ...extra,
  ] as const;

  return (
    <SelectWithLabel
      label={t('convert.output_format_label')}
      menuItems={menuItems}
      onChange={handleOnChange}
      value={fmt}
    />
  );
};
