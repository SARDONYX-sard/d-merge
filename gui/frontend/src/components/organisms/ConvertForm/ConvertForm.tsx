import ClearAllIcon from '@mui/icons-material/ClearAll';
import OutputIcon from '@mui/icons-material/Output';
import { Button, Tooltip } from '@mui/material';
import { open } from '@tauri-apps/plugin-shell';

import { useTranslation } from '@/components/hooks/useTranslation';
import { NOTIFY } from '@/lib/notify';
import { openPath } from '@/services/api/dialog';

import { useConvertContext } from './ConvertProvider';
import { InputField } from './InputWithIcon';
import { PathSelector } from './PathSelector';

import type { ComponentPropsWithRef } from 'react';

export const ConvertForm = () => {
  const { setSelectedFiles, setSelectedDirs, setOutput, setConvertStatuses } = useConvertContext();
  const { t } = useTranslation();

  const handleAllClear = () => {
    setConvertStatuses(new Map());
    setSelectedFiles([]);
    setSelectedDirs([]);
    setOutput('');
  };

  const inputFieldsProps = useInputFieldValues();

  return (
    <>
      <Button
        onClick={handleAllClear}
        startIcon={<ClearAllIcon />}
        sx={{ width: '100%', marginBottom: '15px' }}
        variant='outlined'
      >
        {t('all-clear-btn')}
      </Button>

      {inputFieldsProps.map((inputProps) => {
        return <InputField key={inputProps.label} {...inputProps} />;
      })}

      <PathSelector />
    </>
  );
};

const useInputFieldValues = () => {
  const { output, setOutput } = useConvertContext();
  const { t } = useTranslation();

  const handleOutputClick = () => {
    NOTIFY.asyncTry(async () => {
      await openPath(output, { setPath: setOutput, directory: true });
    });
  };

  const handleOutputIconClick = () => {
    NOTIFY.asyncTry(async () => await open(output));
  };

  return [
    {
      icon: (
        <Tooltip placement='top' title={t('open-output-tooltip')}>
          <OutputIcon
            onClick={handleOutputIconClick}
            sx={{ color: 'action.active', mr: 1, my: 0.5, cursor: 'pointer' }}
          />
        </Tooltip>
      ),
      label: t('output-path'),
      onClick: handleOutputClick,
      path: output,
      setPath: setOutput,
    },
  ] as const satisfies ComponentPropsWithRef<typeof InputField>[];
};
