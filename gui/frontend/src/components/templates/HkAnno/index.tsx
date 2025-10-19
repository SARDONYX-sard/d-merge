'use client';

import { Box, Button } from '@mui/material';
import React, { useCallback, useState } from 'react';
import { NOTIFY } from '@/lib/notify';
import { openPath } from '@/services/api/dialog';
import { Hkanno, hkannoFromText, loadHkanno, NULL_STR, saveHkanno } from '@/services/api/hkanno';
import { OutFormat } from '@/services/api/serde_hkx';
import { ClosableTabs } from './ClosableTabs';
import { FileTab, HkannoTabEditor } from './HkannoTabEditor';
import { useTauriDragDrop } from './useDrag';

function inferFormatFromPath(path: string): OutFormat {
  const p = path.toLowerCase();
  if (p.endsWith('.xml')) return 'xml';
  // heuristic: use hkx => amd64 by default (user can change)
  if (p.endsWith('.hkx')) return 'amd64';
  return 'xml';
}

function inferOutputPath(input: string): string {
  // default: input.basename + ".modified" + ext
  try {
    const idx = input.lastIndexOf('.');
    if (idx === -1) return input + '.modified';
    const base = input.slice(0, idx);
    const ext = input.slice(idx);
    return `${base}.modified${ext}`;
  } catch {
    return input + '.modified';
  }
}

export const HkannoEditorPage: React.FC = () => {
  const [tabs, setTabs] = useState<FileTab[]>([]);
  const [active, setActive] = useState(0);

  // common process: Open file, create tab
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
            // overwrite if already opened
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

  const { dragging } = useTauriDragDrop(openFiles);

  // file select dialog
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
      const parsed = hkannoFromFileTab(tab);
      await saveHkanno(tab.inputPath, tab.outputPath, tab.format, parsed);
      setTabs((prev) => prev.map((t, i) => (i === index ? { ...t, dirty: false, hkanno: parsed } : t)));
      NOTIFY.success('Saved successfully');
    } catch (err) {
      NOTIFY.error('Save failed: ' + String(err));
    }
  };

  return (
    <Box
      component='main'
      sx={{
        display: 'flex',
        flexDirection: 'column',
        minHeight: 'calc(100vh - 56px)',
        position: 'relative',
      }}
    >
      {/* top bar */}
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          px: 1,
          borderBottom: '1px solid #333',
          bgcolor: '#1e1e1e',
        }}
      >
        <ClosableTabs tabs={tabs} active={active} setActive={setActive} setTabs={setTabs} />

        <Box sx={{ flexGrow: 1 }} />
        <Button variant='outlined' color='primary' size='small' onClick={handleOpenClick}>
          Open File
        </Button>
      </Box>

      {/* tabs */}
      {tabs[active] ? (
        <HkannoTabEditor
          tab={tabs[active]}
          onTextChange={(val) =>
            setTabs((prev) => prev.map((t, i) => (i === active ? { ...t, text: val, dirty: true } : t)))
          }
          onOutputChange={(val) =>
            setTabs((prev) => prev.map((t, i) => (i === active ? { ...t, outputPath: val } : t)))
          }
          onFormatChange={(val) => setTabs((prev) => prev.map((t, i) => (i === active ? { ...t, format: val } : t)))}
          onSave={() => saveCurrent(active)}
          onRevert={() => {
            const t = tabs[active];
            if (t.hkanno) {
              setTabs((prev) => prev.map((p, i) => (i === active ? { ...p, text: hkannoToText(p.hkanno!) } : p)));
            }
          }}
        />
      ) : (
        <Box
          sx={{
            flexGrow: 1,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: '#777',
          }}
        >
          Drag and drop the file or click “Open File”.
        </Box>
      )}

      {/* dragging overlay */}
      {dragging && (
        <Box
          sx={{
            position: 'absolute',
            inset: 0,
            backgroundColor: 'rgba(66,165,245,0.15)',
            border: '3px dashed #42a5f5',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: '#42a5f5',
            fontSize: '1.5rem',
            fontWeight: 500,
            zIndex: 1000,
          }}
        >
          Drop HKX or XML files here
        </Box>
      )}
    </Box>
  );
};

// Example text <-> object conversion (frontend-side mirror)
function hkannoToText(h: Hkanno): string {
  const lines: string[] = [];

  lines.push(`# numOriginalFrames: ${h.num_original_frames}`);
  lines.push(`# duration: ${h.duration}`);
  lines.push(`# numAnnotationTracks: ${h.annotation_tracks.length}`);

  for (const track of h.annotation_tracks) {
    if (!track.annotations.length) continue;

    lines.push(`# numAnnotations: ${track.annotations.length}`);
    for (const ann of track.annotations) {
      const text = ann.text ?? NULL_STR;
      lines.push(`${ann.time.toFixed(6)} ${text}`);
    }
  }

  return lines.join('\n');
}

/** Parse hkanno text into frontend Hkanno object */
export const hkannoFromFileTab = (fileTab: FileTab): Hkanno => {
  return {
    ptr: fileTab.ptr,
    num_original_frames: fileTab.num_original_frames,
    duration: fileTab.duration,
    annotation_tracks: hkannoFromText(fileTab.text),
  };
};
