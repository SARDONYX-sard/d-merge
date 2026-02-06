import { Box } from '@mui/material';
import { DragOverlay } from './components/DragOverlay';
import { FileSettingsBar } from './components/FileSettingsBar';
import { HeaderToolbar } from './components/HeaderToolbar';
import { SplitEditors } from './components/SplitEditors';
import { TopBar } from './components/TopBar';
import { HkAnnoEditorProvider } from './context/editorProvider';
import { useTauriDragDrop } from './hooks/useDrag';
import { useHkannoCommands } from './hooks/useHkannoCommands';

/** Public hkanno editor component */
export const HkannoTabEditor = () => {
  return (
    <HkAnnoEditorProvider>
      <HkannoTabEditorInner />
    </HkAnnoEditorProvider>
  );
};

/** Public hkanno editor component */
const HkannoTabEditorInner = () => {
  const { openFiles } = useHkannoCommands();
  const { dragging } = useTauriDragDrop(openFiles);

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: 'calc(100vh - 56px)' }}>
      <TopBar />

      <FileSettingsBar />
      <HeaderToolbar />
      <SplitEditors />
      {dragging && <DragOverlay />}
    </Box>
  );
};
