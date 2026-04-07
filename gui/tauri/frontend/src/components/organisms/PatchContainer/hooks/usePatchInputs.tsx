import OutputIcon from '@mui/icons-material/Output';
import { type SxProps, Tooltip } from '@mui/material';
import { type ComponentPropsWithRef, useCallback, useEffect } from 'react';
import { useTranslation } from '@/components/hooks/useTranslation';
import { usePatchContext } from '@/components/providers/PatchProvider';
import { NOTIFY } from '@/lib/notify';
import { stripGlob } from '@/lib/path';
import { openPath } from '@/services/api/dialog';
import { getSkyrimDir } from '@/services/api/patch';
import { openPath as open } from '@/services/api/shell';

import type { InputField } from '@/components/molecules/InputField/InputField';

const sx: SxProps = { color: 'action.active', mr: 1, my: 0.5, cursor: 'pointer' };

export const usePatchInputs = () => {
  const {
    output,
    setOutput,

    isVfsMode,
    patchOptions,
    setPatchOptions,

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
      } else {
        setSkyrimDataDir(path);
      }
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
      } catch (e) {
        NOTIFY.error(t('patch.autoDetectSkyrimData_error_massage'));
        if (e instanceof Error) console.error(e);
      }
    };

    fetchDir().catch((e) => NOTIFY.error(`${e}`));
  }, [isVfsMode, patchOptions.outputTarget, setVfsSkyrimDataDir, t]);

  const inputHandlers = {
    onClick: async () =>
      await NOTIFY.asyncTry(async () => await openPath(stripGlob(dataDir), { setPath: setDataDir, directory: true })),
    onIconClick: async () => await NOTIFY.asyncTry(async () => await open(stripGlob(dataDir))),
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
      disabled: isVfsMode,
      label: `${patchOptions.outputTarget} ${t('patch.input_directory')}`,
      onClick: inputHandlers.onClick,
      path: dataDir,
      placeholder,
      setPath: setDataDir,
      onChange: (_event) => {
        setPatchOptions((prev) => ({ ...prev, skyrimDataDirGlob: isVfsMode ? vfsSkyrimDataDir : skyrimDataDir }));
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
