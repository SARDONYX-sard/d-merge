import { Box, Button, FormControl, InputLabel, MenuItem, Select, TextField, Typography } from '@mui/material';
import { Allotment } from 'allotment';
import React from 'react';
import 'allotment/dist/style.css';
import { MonacoEditor } from '@/components/organisms/MonacoEditor';
import { useEditorModeContext } from '@/components/providers/EditorModeProvider';
import { Hkanno } from '@/services/api/hkanno';
import { OutFormat } from '@/services/api/serde_hkx';
import { hkannoFromFileTab } from '.';

export type FileTab = {
  id: string;
  inputPath: string;
  outputPath: string;
  format: OutFormat;

  /** XML index e.g. `#0003`  */
  ptr: string;
  num_original_frames: number;
  duration: number;
  /** Hkanno.AnnotationTrack[] */
  text: string;
  /** file first loaded original hkanno(use on revert). readonly */
  hkanno: Readonly<Hkanno>;

  dirty?: boolean;
};

/* -------------------------------------------------------------------------- */
/*                               Sub Components                               */
/* -------------------------------------------------------------------------- */

function HeaderToolbar({
  onSave,
  onRevert,
  showPreview,
  setShowPreview,
}: {
  onSave: () => void;
  onRevert: () => void;
  showPreview: boolean;
  setShowPreview: (v: boolean) => void;
}) {
  return (
    <Box
      sx={{
        display: 'flex',
        alignItems: 'center',
        gap: 1,
        px: 1,
        py: 0.5,
        borderBottom: '1px solid #333',
        bgcolor: '#1e1e1e',
      }}
    >
      <Button variant='contained' onClick={onSave}>
        Save
      </Button>
      <Button variant='outlined' onClick={onRevert}>
        Revert
      </Button>
      <Box sx={{ flexGrow: 1 }} />
      <Button variant='text' onClick={() => setShowPreview(!showPreview)}>
        {showPreview ? 'Hide Preview' : 'Show Preview'}
      </Button>
    </Box>
  );
}

function FileSettingsBar({
  outputPath,
  format,
  onOutputChange,
  onFormatChange,
}: {
  outputPath: string;
  format: OutFormat;
  onOutputChange: (v: string) => void;
  onFormatChange: (v: OutFormat) => void;
}) {
  return (
    <Box
      sx={{
        display: 'flex',
        gap: 2,
        alignItems: 'center',
        px: 2,
        py: 1,
        borderBottom: '1px solid #444',
        bgcolor: '#2a2a2a',
      }}
    >
      <TextField
        label='Output Path'
        value={outputPath}
        onChange={(e) => onOutputChange(e.target.value)}
        size='small'
        fullWidth
      />
      <FormControl size='small' sx={{ minWidth: 120 }}>
        <InputLabel id='format-label'>Format</InputLabel>
        <Select
          labelId='format-label'
          value={format}
          label='Format'
          onChange={(e) => onFormatChange(e.target.value as OutFormat)}
        >
          <MenuItem value='amd64'>amd64</MenuItem>
          <MenuItem value='win32'>win32</MenuItem>
          <MenuItem value='xml'>xml</MenuItem>
        </Select>
      </FormControl>
    </Box>
  );
}

function SplitEditors({
  tab,
  isVimMode,
  showPreview,
  onTextChange,
}: {
  tab: FileTab;
  isVimMode: boolean;
  showPreview: boolean;
  onTextChange: (v: string) => void;
}) {
  return (
    <Allotment>
      {/* Left: Annotation editor */}
      <Allotment.Pane minSize={300}>
        <Box sx={{ height: '100%' }}>
          <Typography variant='subtitle2' sx={{ px: 2, pt: 1, color: '#aaa' }}>
            Annotation
          </Typography>
          <MonacoEditor
            height='calc(100% - 24px)'
            defaultLanguage='hkanno' // NOTE: Comments starting with `#` are being used as pseudo-comments.
            value={tab.text}
            onChange={(val) => val && onTextChange(val)}
            options={{ minimap: { enabled: true }, fontSize: 13 }}
            vimMode={isVimMode}
          />
        </Box>
      </Allotment.Pane>

      {/* Right: Preview */}
      {showPreview && (
        <Allotment.Pane minSize={200} preferredSize={480}>
          <Box sx={{ height: '100%' }}>
            <Typography variant='subtitle2' sx={{ px: 2, pt: 1, color: '#aaa' }}>
              Preview
            </Typography>
            <MonacoEditor
              height='calc(100% - 24px)'
              defaultLanguage='json'
              value={JSON.stringify(hkannoFromFileTab(tab), null, 2)}
              options={{
                readOnly: true,
                minimap: { enabled: false },
                fontSize: 13,
              }}
            />
          </Box>
        </Allotment.Pane>
      )}
    </Allotment>
  );
}

/* -------------------------------------------------------------------------- */
/*                               Main Component                               */
/* -------------------------------------------------------------------------- */

export const HkannoTabEditor: React.FC<{
  tab: FileTab;
  onTextChange: (val: string) => void;
  onOutputChange: (val: string) => void;
  onFormatChange: (val: OutFormat) => void;
  onSave: () => void;
  onRevert: () => void;
}> = ({ tab, onTextChange, onOutputChange, onFormatChange, onSave, onRevert }) => {
  const { editorMode } = useEditorModeContext();
  const [showPreview, setShowPreview] = React.useState(false);
  const isVimMode = editorMode === 'vim';

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: 'calc(100vh - 56px)' }}>
      <HeaderToolbar onSave={onSave} onRevert={onRevert} showPreview={showPreview} setShowPreview={setShowPreview} />
      <FileSettingsBar
        outputPath={tab.outputPath}
        format={tab.format}
        onOutputChange={onOutputChange}
        onFormatChange={onFormatChange}
      />
      <SplitEditors tab={tab} isVimMode={isVimMode} showPreview={showPreview} onTextChange={onTextChange} />
    </Box>
  );
};
