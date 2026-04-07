import { useEffect, useState } from 'react';
import { useDebouncedCallback } from '@/components/hooks/useDebounceCallback';
import { usePatchContext } from '@/components/providers/PatchProvider';
import { loadModsInfo } from '@/services/api/patch';
import { mergeModInfoList } from '@/services/api/patch/mod_item';

export const useFetchModInfo = () => {
  const { isVfsMode, vfsSkyrimDataDir, skyrimDataDir, setModList, setVfsModList, setFetchIsEmpty } = usePatchContext();

  const [loading, setLoading] = useState(false);

  const fetchMods = useDebouncedCallback(async (dir: string, mode: boolean) => {
    if (!dir.trim()) return;

    setLoading(true);

    try {
      const fetched = await loadModsInfo(dir.trim(), mode);

      if (fetched.length > 0) {
        setFetchIsEmpty(false);

        if (mode) {
          setVfsModList((prev) => mergeModInfoList(prev, fetched));
        } else {
          setModList((prev) => mergeModInfoList(prev, fetched));
        }
      } else {
        setFetchIsEmpty(true);
      }
    } finally {
      setLoading(false);
    }
  }, 450);

  useEffect(() => {
    const dir = isVfsMode ? vfsSkyrimDataDir : skyrimDataDir;

    fetchMods(dir, isVfsMode);
  }, [isVfsMode, vfsSkyrimDataDir, skyrimDataDir]);

  return { loading };
};
