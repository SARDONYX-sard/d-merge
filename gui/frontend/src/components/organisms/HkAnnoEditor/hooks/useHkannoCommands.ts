import { useCallback } from 'react';
import { NOTIFY } from '@/lib/notify';
import { openPath } from '@/services/api/dialog';
import { hkannoFromText, hkannoToText, loadHkanno, saveHkanno } from '@/services/api/hkanno';
import type { OutFormat } from '@/services/api/serde_hkx';
import { useEditorContext } from '../context/editorContext';
import { FileTab } from '../types/FileTab';

/** Editor side-effect commands */
export const useHkannoCommands = () => {
  const [state, dispatch] = useEditorContext();

  const openFiles = useCallback(
    async (paths: string[]) => {
      const opened: FileTab[] = [];

      for (const path of paths) {
        try {
          const hkanno = await loadHkanno(path);
          opened.push({
            id: path,
            inputPath: path,
            outputPath: inferOutputPath(path),
            format: inferFormatFromPath(path),
            ptr: hkanno.ptr,
            duration: hkanno.duration,
            num_original_frames: hkanno.num_original_frames,
            text: hkannoToText(hkanno),
            hkanno,
          });
        } catch (e) {
          NOTIFY.error(`Failed to load: ${path}`);
        }
      }

      if (opened.length) {
        dispatch({ type: 'OPEN_TABS', tabs: opened });
      }
    },
    [dispatch],
  );

  const handleOpenClick = async () => {
    const selected = await openPath('', {
      multiple: true,
      filters: [{ name: 'Havok Animation Files', extensions: ['hkx', 'xml'] }],
    });
    if (selected) {
      openFiles(Array.isArray(selected) ? selected : [selected]);
    }
  };

  const saveCurrent = async () => {
    const tab = state.tabs[state.active];
    if (!tab) return;

    try {
      const parsed = {
        ptr: tab.ptr,
        num_original_frames: tab.num_original_frames,
        duration: tab.duration,
        annotation_tracks: hkannoFromText(tab.text),
      };
      await saveHkanno(tab.inputPath, tab.outputPath, tab.format, parsed);
      dispatch({ type: 'MARK_SAVED', index: state.active, hkanno: parsed });
      NOTIFY.success('Saved successfully');
    } catch (e) {
      NOTIFY.error('Save failed');
    }
  };

  return { openFiles, handleOpenClick, saveCurrent };
};

const inferFormatFromPath = (path: string): OutFormat => (path.toLowerCase().endsWith('.hkx') ? 'amd64' : 'xml');

const inferOutputPath = (input: string) => {
  const i = input.lastIndexOf('.');
  return i === -1 ? input + '.modified' : `${input.slice(0, i)}.modified${input.slice(i)}`;
};
