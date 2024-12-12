import OutputIcon from '@mui/icons-material/Output';
import { type SxProps, Tooltip } from '@mui/material';
import { open } from '@tauri-apps/plugin-shell';

import { useTranslation } from '@/components/hooks/useTranslation';
import type { InputField } from '@/components/molecules/InputField/InputField';
import { NOTIFY } from '@/lib/notify';
import { openPath } from '@/services/api/dialog';

import { usePatchContext } from './PatchProvider';

import type { ComponentPropsWithRef } from 'react';

const sx: SxProps = { color: 'action.active', mr: 1, my: 0.5, cursor: 'pointer' };

const createHandlers = (path: string, setPath: (path: string) => void) => ({
  onClick: () => NOTIFY.asyncTry(async () => await openPath(path, { setPath, directory: true })),
  onIconClick: () => NOTIFY.asyncTry(async () => await open(path)),
});

export const usePatchInputs = () => {
  const { modInfoDir, setModInfoDir, output, setOutput } = usePatchContext();
  const { t } = useTranslation();

  const inputHandlers = createHandlers(modInfoDir, setModInfoDir);
  const outputHandlers = createHandlers(output, setOutput);

  return [
    {
      icon: (
        <Tooltip placement='top' title={t('open-tooltip')}>
          <OutputIcon onClick={inputHandlers.onIconClick} sx={sx} />
        </Tooltip>
      ),
      label: 'Skyrim Data Dir',
      onClick: inputHandlers.onClick,
      path: modInfoDir,
      setPath: setModInfoDir,
      placeholder: 'D:/Steam/steamapps/common/Skyrim Special Edition/Data',
    },
    {
      icon: (
        <Tooltip placement='top' title={t('open-output-tooltip')}>
          <OutputIcon onClick={outputHandlers.onIconClick} sx={sx} />
        </Tooltip>
      ),
      label: t('output-path'),
      onClick: outputHandlers.onClick,
      path: output,
      setPath: setOutput,
    },
  ] as const satisfies ComponentPropsWithRef<typeof InputField>[];
};
