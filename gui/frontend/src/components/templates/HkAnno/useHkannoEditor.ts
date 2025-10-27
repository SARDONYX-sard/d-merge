import { useCallback } from 'react';
import z from 'zod';
import { useStorageState } from '@/components/hooks/useStorageState';
import { NOTIFY } from '@/lib/notify';
import { PRIVATE_CACHE_OBJ } from '@/lib/storage/cacheKeys';
import { openPath } from '@/services/api/dialog';
import { hkannoFromText, hkannoToText, loadHkanno, saveHkanno } from '@/services/api/hkanno';
import type { OutFormat } from '@/services/api/serde_hkx';
import { FileTabSchema } from './HkannoTabEditor';

export const useHkannoEditor = () => {
  const [tabs, setTabs] = useStorageState(PRIVATE_CACHE_OBJ.hkannoFileTabs, z.array(FileTabSchema).catch([]));
  const [active, setActive] = useStorageState(PRIVATE_CACHE_OBJ.hkannoActiveTab, z.number().catch(0));
  const [showPreview, setShowPreview] = useStorageState(PRIVATE_CACHE_OBJ.hkannoShowPreview, z.boolean().catch(false));

  const openFiles = useCallback(
    async (paths: string[]) => {
      for (const path of paths) {
        const ext = path.split('.').pop()?.toLowerCase();
        if (!['hkx', 'xml'].includes(ext ?? '')) continue;
        try {
          const hkanno = await loadHkanno(path);
          const text = hkannoToText(hkanno);
          const { ptr, num_original_frames, duration } = hkanno;

          setTabs((prev) => {
            const existing = prev.findIndex((t) => t.inputPath === path);
            const next = [...prev];
            if (existing >= 0) {
              next[existing] = { ...next[existing], text, ptr, num_original_frames, duration };
              setActive(existing);
              return next;
            }
            return [
              ...next,
              {
                id: path,
                inputPath: path,
                outputPath: inferOutputPath(path),
                format: inferFormatFromPath(path),
                ptr,
                duration,
                num_original_frames,
                text,
                hkanno,
              },
            ];
          });
          setActive((_prev) => tabs.length);
        } catch (e) {
          NOTIFY.error(`Failed to load: ${path} ${e}`);
        }
      }
    },
    [tabs.length],
  );

  const handleOpenClick = useCallback(async () => {
    const selected = await openPath('', {
      multiple: true,
      filters: [{ name: 'Havok Animation Files', extensions: ['hkx', 'xml'] }],
    });
    if (selected) {
      const paths = Array.isArray(selected) ? selected : [selected];
      await openFiles(paths);
    }
  }, [openFiles]);

  const saveCurrent = async (index: number) => {
    const tab = tabs.at(index);
    if (!tab) return;
    try {
      const parsed = {
        ptr: tab.ptr,
        num_original_frames: tab.num_original_frames,
        duration: tab.duration,
        annotation_tracks: hkannoFromText(tab.text),
      };
      await saveHkanno(tab.inputPath, tab.outputPath, tab.format, parsed);
      setTabs((prev) => prev.map((t, i) => (i === index ? { ...t, dirty: false, hkanno: parsed } : t)));
      NOTIFY.success('Saved successfully');
    } catch (err) {
      NOTIFY.error('Save failed: ' + String(err));
    }
  };

  return {
    tabs,
    setTabs,
    active,
    setActive,
    showPreview,
    setShowPreview,
    openFiles,
    handleOpenClick,
    saveCurrent,
  };
};

const inferFormatFromPath = (path: string): OutFormat => {
  const p = path.toLowerCase();
  if (p.endsWith('.xml')) return 'xml';
  if (p.endsWith('.hkx')) return 'amd64';
  return 'xml';
};

const inferOutputPath = (input: string): string => {
  // default: input.basename + ".modified" + ext
  const idx = input.lastIndexOf('.');
  if (idx === -1) return input + '.modified';
  const base = input.slice(0, idx);
  const ext = input.slice(idx);
  return `${base}.modified${ext}`;
};
