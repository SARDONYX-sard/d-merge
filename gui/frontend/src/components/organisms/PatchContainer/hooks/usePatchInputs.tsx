import OutputIcon from '@mui/icons-material/Output';
import { Checkbox, type SxProps, Tooltip } from '@mui/material';
import { type ComponentPropsWithRef, useEffect } from 'react';

import { useDebounce } from '@/components/hooks/useDebounce';
import { useTranslation } from '@/components/hooks/useTranslation';
import type { InputField } from '@/components/molecules/InputField/InputField';
import { usePatchContext } from '@/components/providers/PatchProvider';
import { NOTIFY } from '@/lib/notify';
import { stripGlob } from '@/lib/path';
import { openPath } from '@/services/api/dialog';
import { getSkyrimDir } from '@/services/api/patch';
import { openPath as open } from '@/services/api/shell';

const sx: SxProps = { color: 'action.active', mr: 1, my: 0.5, cursor: 'pointer' };

export const usePatchInputs = () => {
  const {
    cacheModInfoDir,
    setCacheModInfoDir,
    modInfoDir,
    setModInfoDir,
    autoDetectEnabled,
    setAutoDetectEnabled,
    output,
    setOutput,
    patchOptions,
  } = usePatchContext();
  const { t } = useTranslation();

  const deferredAutoDetectEnabled = useDebounce(autoDetectEnabled, 450);

  // If a setState with a branch is not wrapped in useEffect, purity(Returns same value) is lost and an error occurs.
  useEffect(() => {
    if (!deferredAutoDetectEnabled) {
      setModInfoDir(cacheModInfoDir);
    }
  }, [cacheModInfoDir, deferredAutoDetectEnabled, setModInfoDir]);

  useEffect(() => {
    if (!deferredAutoDetectEnabled) {
      return;
    }

    const fetchDir = async () => {
      try {
        const dir = await getSkyrimDir(patchOptions.outputTarget);
        setModInfoDir(dir);
      } catch (_) {
        NOTIFY.error(t('patch.autoDetectSkyrimData_error_massage'));
      }
    };

    fetchDir();
  }, [deferredAutoDetectEnabled, patchOptions.outputTarget, setModInfoDir, t]);

  const inputHandlers = {
    onClick: () =>
      NOTIFY.asyncTry(async () => await openPath(stripGlob(modInfoDir), { setPath: setModInfoDir, directory: true })),
    onIconClick: () => NOTIFY.asyncTry(async () => await open(stripGlob(modInfoDir))),
    onCheckboxToggle: () => {
      setAutoDetectEnabled((prev) => !prev);
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
      setPath: (path) => {
        if (!autoDetectEnabled) {
          setCacheModInfoDir(path);
        }
        setModInfoDir(path);
      },
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
