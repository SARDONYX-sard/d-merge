import ClearAllIcon from '@mui/icons-material/ClearAll';
import OutputIcon from '@mui/icons-material/Output';
import { Button, Tooltip } from '@mui/material';
import type { ComponentPropsWithRef } from 'react';
import { useTranslation } from '@/components/hooks/useTranslation';
import { InputField } from '@/components/molecules/InputField/InputField';
import { NOTIFY } from '@/lib/notify';
import { openPath } from '@/services/api/dialog';
import { openPath as open } from '@/services/api/shell';
import { CONVERT_TREE_INIT_VALUES, useConvertContext } from './ConvertProvider';
import { PathSelector } from './PathSelector';
import { PathSelectorButtons } from './PathSelectorButtons';

export const ConvertForm = () => {
  const { setSelectedFiles, setSelectedDirs, setSelectedTree, setOutput, setConvertStatuses, selectionType } =
    useConvertContext();
  const { t } = useTranslation();

  const handleAllClear = () => {
    setConvertStatuses(new Map());
    switch (selectionType) {
      case 'files':
        setSelectedFiles([]);
        break;
      case 'dir':
        setSelectedDirs([]);
        break;
      case 'tree':
        setSelectedTree(CONVERT_TREE_INIT_VALUES);
        break;
      default:
        break;
    }
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
        {t('general.all_clear_button')}
      </Button>

      {inputFieldsProps.map((inputProps) => {
        return <InputField key={inputProps.label} {...inputProps} />;
      })}

      <PathSelectorButtons />

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
        <Tooltip placement='top' title={t('output.open_tooltip')}>
          <OutputIcon
            onClick={handleOutputIconClick}
            sx={{ color: 'action.active', mr: 1, my: 0.5, cursor: 'pointer' }}
          />
        </Tooltip>
      ),
      label: t('output.path_label'),
      onClick: handleOutputClick,
      path: output,
      setPath: setOutput,
    },
  ] as const satisfies ComponentPropsWithRef<typeof InputField>[];
};
