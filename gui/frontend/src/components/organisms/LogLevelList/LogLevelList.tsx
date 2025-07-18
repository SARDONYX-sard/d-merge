import type { SelectChangeEvent } from '@mui/material';
import { useCallback } from 'react';
import { useTranslation } from '@/components/hooks/useTranslation';
import { SelectWithLabel } from '@/components/molecules/SelectWithLabel';
import { useLogLevelContext } from '@/components/providers/LogLevelProvider';
import { NOTIFY } from '@/lib/notify';
import { LOG } from '@/services/api/log';

export const LogLevelList = () => {
  const { logLevel, setLogLevel } = useLogLevelContext();

  const handleOnChange = useCallback(
    async ({ target }: SelectChangeEvent) => {
      const newLogLevel = LOG.normalize(target.value);
      setLogLevel(newLogLevel);
      await NOTIFY.asyncTry(async () => await LOG.changeLevel(newLogLevel));
    },
    [setLogLevel],
  );

  const menuItems = [
    { value: 'trace', label: 'Trace' },
    { value: 'debug', label: 'Debug' },
    { value: 'info', label: 'Info' },
    { value: 'warn', label: 'Warning' },
    { value: 'error', label: 'Error' },
  ] as const;

  return (
    <SelectWithLabel
      label={useTranslation().t('log_level.list_label')}
      menuItems={menuItems}
      onChange={handleOnChange}
      value={logLevel}
    />
  );
};
