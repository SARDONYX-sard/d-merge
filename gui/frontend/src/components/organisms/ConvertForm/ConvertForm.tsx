import ClearAllIcon from '@mui/icons-material/ClearAll';
import OutputIcon from '@mui/icons-material/Output';
import { Button } from '@mui/material';

import { useTranslation } from '@/components/hooks/useTranslation';
import { NOTIFY } from '@/lib/notify';
import { openPath } from '@/services/api/dialog';

import type { ComponentPropsWithRef } from 'react';
import { useConvertContext } from './ConvertProvider';
import { InputField } from './InputWithIcon';
import { PathSelector } from './PathSelector';

export const ConvertForm = () => {
  const { setSelectedFiles, setOutput } = useConvertContext();
  const { t } = useTranslation();

  const handleAllClear = () => {
    setSelectedFiles([]);
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

  const handleOutputClick = () => {
    NOTIFY.asyncTry(async () => {
      await openPath(output, { setPath: setOutput, directory: true });
    });
  };

  return [
    {
      icon: <OutputIcon sx={{ color: 'action.active', mr: 1, my: 0.5 }} />,
      label: 'Output',
      onClick: handleOutputClick,
      path: output,
      setPath: setOutput,
    },
  ] as const satisfies ComponentPropsWithRef<typeof InputField>[];
};
