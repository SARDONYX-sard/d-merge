import OutputIcon from '@mui/icons-material/Output';
import { type SxProps, Tooltip } from '@mui/material';

import { useTranslation } from '@/components/hooks/useTranslation';
import type { InputField } from '@/components/molecules/InputField/InputField';
import { NOTIFY } from '@/lib/notify';
import { openPath } from '@/services/api/dialog';
import { openPath as open } from '@/services/api/shell';

import { usePatchContext } from './PatchProvider';

import type { ComponentPropsWithRef } from 'react';

const sx: SxProps = { color: 'action.active', mr: 1, my: 0.5, cursor: 'pointer' };

export const usePatchInputs = () => {
  const { modInfoDir, setModInfoDir, output, setOutput } = usePatchContext();
  const { t } = useTranslation();

  const inputHandlers = {
    onClick: () => NOTIFY.asyncTry(async () => await openPath(modInfoDir, { setPath: setModInfoDir, directory: true })),
    onIconClick: () => NOTIFY.asyncTry(async () => await open(modInfoDir)),
  };

  const outputHandlers = {
    onClick: () => NOTIFY.asyncTry(async () => await openPath(output, { setPath: setOutput, directory: true })),
    onIconClick: () => NOTIFY.asyncTry(async () => await open(output)),
  };

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
