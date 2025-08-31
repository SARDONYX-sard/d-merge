import OutputIcon from '@mui/icons-material/Output';
import { Checkbox, type SxProps, Tooltip } from '@mui/material';
import { type ComponentPropsWithRef, useCallback, useEffect } from 'react';

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
    output,
    setOutput,

    isVfsMode,
    setIsVfsMode,
    patchOptions,

    vfsSkyrimDataDir,
    setVfsSkyrimDataDir,

    skyrimDataDir,
    setSkyrimDataDir,
  } = usePatchContext();
  const { t } = useTranslation();

  const dataDir = isVfsMode ? vfsSkyrimDataDir : skyrimDataDir;
  const setDataDir = useCallback(
    (path: string) => {
      if (isVfsMode) {
        setVfsSkyrimDataDir(path);
      }
      setSkyrimDataDir(path);
    },
    [setVfsSkyrimDataDir, setSkyrimDataDir],
  );

  useEffect(() => {
    if (!isVfsMode) {
      return;
    }

    const fetchDir = async () => {
      try {
        setVfsSkyrimDataDir(await getSkyrimDir(patchOptions.outputTarget));
      } catch (_) {
        NOTIFY.error(t('patch.autoDetectSkyrimData_error_massage'));
      }
    };

    fetchDir();
  }, [isVfsMode, patchOptions.outputTarget, setVfsSkyrimDataDir, t]);

  const inputHandlers = {
    onClick: async () => {
      return await NOTIFY.asyncTry(
        async () => await openPath(stripGlob(dataDir), { setPath: setDataDir, directory: true }),
      );
    },
    onIconClick: async () => await NOTIFY.asyncTry(async () => await open(stripGlob(dataDir))),
    onCheckboxToggle: () => {
      setIsVfsMode((prev) => !prev);
    },
  };

  const outputHandlers = {
    onClick: () => NOTIFY.asyncTry(async () => await openPath(output, { setPath: setOutput, directory: true })),
    onIconClick: () => NOTIFY.asyncTry(async () => await open(output)),
  };

  const placeholder = isVfsMode
    ? 'D:/Steam/steamapps/common/Skyrim Special Edition/Data'
    : 'D:\\GAME\\ModOrganizer Skyrim SE\\mods\\*';

  return [
    {
      icon: (
        <Tooltip placement='auto-end' sx={sx} title={t('directory.open_tooltip')}>
          <OutputIcon onClick={inputHandlers.onIconClick} />
        </Tooltip>
      ),
      endIcon: (
        <Tooltip placement='top' title={t('patch.autoDetectSkyrimData_tooltip')}>
          <Checkbox checked={isVfsMode} onChange={inputHandlers.onCheckboxToggle} />
        </Tooltip>
      ),
      disabled: isVfsMode,
      label: `${patchOptions.outputTarget} ${t('patch.input_directory')}`,
      onClick: inputHandlers.onClick,
      path: dataDir,
      placeholder,
      setPath: setDataDir,
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
