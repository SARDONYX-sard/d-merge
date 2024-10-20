import ClearAllIcon from '@mui/icons-material/ClearAll';
import InputIcon from '@mui/icons-material/Input';
import OutputIcon from '@mui/icons-material/Output';
import { Button, Grid2 } from '@mui/material';
import { save } from '@tauri-apps/plugin-dialog';

import { useTranslation } from '@/components/hooks/useTranslation';
import { NOTIFY } from '@/lib/notify';
import { openPath } from '@/services/api/dialog';

import { useConvertContext } from './ConvertProvider';
import { DirModeList } from './DirModeList';
import { InputWithIcon } from './InputWithIcon';
import { OutFormatList } from './OutFormatList';

export const ConvertForm = () => {
  const { pathMode, input, setInput, output, setOutput } = useConvertContext();
  const { t } = useTranslation();

  const handleAllClear = () => {
    setInput('');
    setOutput('');
  };

  const isDir = pathMode === 'dir';

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
      <InputWithIcon
        icon={<InputIcon sx={{ color: 'action.active', mr: 1, my: 0.5 }} />}
        label={'Input'}
        onClick={() => {
          NOTIFY.asyncTry(async () => await openPath(input, { setPath: setInput, directory: isDir }));
        }}
        path={input}
        setPath={setInput}
      />
      <InputWithIcon
        icon={<OutputIcon sx={{ color: 'action.active', mr: 1, my: 0.5 }} />}
        label={'Output'}
        onClick={() => {
          NOTIFY.asyncTry(async () => {
            if (isDir) {
              await openPath(output, { setPath: setOutput, directory: isDir });
            } else {
              const path = await save({ defaultPath: output });
              if (path) {
                setOutput(path);
              }
            }
          });
        }}
        path={output}
        setPath={setOutput}
      />
      <Grid2 sx={{ display: 'flex', justifyContent: 'flex-end' }}>
        <OutFormatList />
        <DirModeList />
      </Grid2>
    </>
  );
};
