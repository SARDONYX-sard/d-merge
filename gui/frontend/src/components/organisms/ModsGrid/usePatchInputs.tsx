import OutputIcon from '@mui/icons-material/Output';
import { Tooltip } from '@mui/material';
import { open } from '@tauri-apps/plugin-shell';

import { useTranslation } from '@/components/hooks/useTranslation';
import type { InputField } from '@/components/molecules/InputField/InputField';
import { NOTIFY } from '@/lib/notify';
import { openPath } from '@/services/api/dialog';

import { usePatchContext } from './PatchProvider';

import type { ComponentPropsWithRef } from 'react';

export const usePatchInputs = () => {
  const { modInfoDir, setModInfoDir, output, setOutput } = usePatchContext();
  const { t } = useTranslation();

  const handleInputClick = () => {
    NOTIFY.asyncTry(async () => {
      await openPath(modInfoDir, { setPath: setModInfoDir, directory: true });
    });
  };

  const handleInputIconClick = () => {
    NOTIFY.asyncTry(async () => await open(modInfoDir));
  };

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
        <Tooltip placement='top' title={"Open specified directory."}>
          <OutputIcon
            onClick={handleInputIconClick}
            sx={{ color: 'action.active', mr: 1, my: 0.5, cursor: 'pointer' }}
          />
        </Tooltip>
      ),
      label: 'Mod Dir',
      onClick: handleInputClick,
      path: modInfoDir,
      setPath: setModInfoDir,
      placeholder: 'D:/GAME/ModOrganizer Skyrim SE/mods/*/Nemesis_Engine/mod',
    },
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
