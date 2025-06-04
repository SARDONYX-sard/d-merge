import OutputIcon from '@mui/icons-material/Output';
import { Checkbox, type SxProps, Tooltip } from '@mui/material';
import { useEffect, type ComponentPropsWithRef } from 'react';

import { useTranslation } from '@/components/hooks/useTranslation';
import type { InputField } from '@/components/molecules/InputField/InputField';
import { NOTIFY } from '@/lib/notify';
import { stripGlob } from '@/lib/path';
import { openPath } from '@/services/api/dialog';
import { getSkyrimDir } from '@/services/api/patch';
import { openPath as open } from '@/services/api/shell';

import { usePatchContext } from './PatchProvider';

const sx: SxProps = { color: 'action.active', mr: 1, my: 0.5, cursor: 'pointer' };

export const usePatchInputs = () => {
  const {
    modInfoDir,
    setModInfoDir,
    modInfoDirPrev,
    setModInfoDirPrev,
    autoDetectEnabled,
    setAutoDetectEnabled,
    output,
    setOutput,
    patchOptions,
  } = usePatchContext();
  const { t } = useTranslation();

  useEffect(() => {
    if (!autoDetectEnabled) {
      return;
    }

    const fetchDir = async () => {
      try {
        const dir = await getSkyrimDir(patchOptions.outputTarget);
        setModInfoDir(dir);
      } catch (_error) {
        throw new Error(t('patch.autoDetectSkyrimData_error_massage'));
      }
    };

    NOTIFY.asyncTry(fetchDir);
  }, [autoDetectEnabled, patchOptions.outputTarget, setModInfoDir, t]);

  const inputHandlers = {
    onClick: () =>
      NOTIFY.asyncTry(async () => await openPath(stripGlob(modInfoDir), { setPath: setModInfoDir, directory: true })),
    onIconClick: () => NOTIFY.asyncTry(async () => await open(stripGlob(modInfoDir))),
    onCheckboxToggle: () => {
      setAutoDetectEnabled((prev) => {
        const autoDetect = !prev;
        if (autoDetect) {
          setModInfoDirPrev(modInfoDir);
        } else {
          setModInfoDir(modInfoDirPrev);
        }
        return autoDetect;
      });
    },
  };

  const outputHandlers = {
    onClick: () => NOTIFY.asyncTry(async () => await openPath(output, { setPath: setOutput, directory: true })),
    onIconClick: () => NOTIFY.asyncTry(async () => await open(output)),
  };

  return [
    {
      icon: (
        <Tooltip placement='auto-end' sx={sx} title={t('directory.open_tooltip')}>
          <OutputIcon onClick={inputHandlers.onIconClick} />
        </Tooltip>
      ),
      endIcon: (
        <Tooltip placement='top' title={t('patch.autoDetectSkyrimData_tooltip')}>
          <Checkbox checked={autoDetectEnabled} onChange={inputHandlers.onCheckboxToggle} />
        </Tooltip>
      ),
      disabled: autoDetectEnabled,
      label: `${patchOptions.outputTarget} ${t('patch.input_directory')}`,
      onClick: inputHandlers.onClick,
      path: modInfoDir,
      placeholder: 'D:/Steam/steamapps/common/Skyrim Special Edition/Data',
      setPath: setModInfoDir,
    },
    {
      icon: (
        <Tooltip placement='auto-end' sx={sx} title={t('output.open_tooltip')}>
          <OutputIcon onClick={outputHandlers.onIconClick} />
        </Tooltip>
      ),
      label: t('output.path_label'),
      onClick: outputHandlers.onClick,
      path: output,
      setPath: setOutput,
    },
  ] as const satisfies ComponentPropsWithRef<typeof InputField>[];
};
