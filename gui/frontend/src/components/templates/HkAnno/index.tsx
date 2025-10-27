'use client';

import { Box, Button } from '@mui/material';
import React from 'react';
import { useInjectJs } from '@/components/hooks/useInjectJs';
import { hkannoToText } from '@/services/api/hkanno';
import { OutFormat } from '@/services/api/serde_hkx';
import { ClosableTabs } from './ClosableTabs';
import { HkannoTabEditor } from './HkannoTabEditor';
import { useTauriDragDrop } from './useDrag';
import { useHkannoEditor } from './useHkannoEditor';

export const HkannoEditorPage: React.FC = () => {
  useInjectJs();

  const { tabs, setTabs, active, setActive, showPreview, setShowPreview, handleOpenClick, openFiles, saveCurrent } =
    useHkannoEditor();
  const { dragging } = useTauriDragDrop(openFiles);

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
          setShowPreview={setShowPreview}
          showPreview={showPreview}
          onTextChange={(val) =>
            setTabs((prev) => prev.map((t, i) => (i === active ? { ...t, text: val, dirty: true } : t)))
          }
          onCursorChange={(pos) => setTabs((prev) => prev.map((t, i) => (i === active ? { ...t, cursorPos: pos } : t)))}
          onOutputChange={(val) =>
            setTabs((prev) => prev.map((t, i) => (i === active ? { ...t, outputPath: val } : t)))
          }
          onFormatChange={(format) =>
            setTabs((prev) =>
              prev.map((t, i) =>
                i === active ? { ...t, format, outputPath: changeExtension(t.outputPath, format) } : t,
              ),
            )
          }
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

const changeExtension = (outputPath: string, format: OutFormat): string => {
  const idx = outputPath.lastIndexOf('.');
  const base = idx === -1 ? outputPath : outputPath.slice(0, idx);
  switch (format) {
    case 'amd64':
    case 'win32':
      return `${base}.hkx`;
    case 'xml':
      return `${base}.xml`;
    case 'json':
      return `${base}.json`;
    default:
      return outputPath;
  }
};
