import { Box, Button, FormControl, InputLabel, MenuItem, Select, TextField, Typography } from '@mui/material';
import { Allotment } from 'allotment';
import React, { useState } from 'react';
import { useMonacoSyncJump } from './useMonacoSyncJump';
import 'allotment/dist/style.css';
import z from 'zod';
import { outFormatSchema } from '@/components/organisms/ConvertForm/schemas/out_format';
import { MonacoEditor } from '@/components/organisms/MonacoEditor';
import { useEditorModeContext } from '@/components/providers/EditorModeProvider';
import { HkannoSchema, previewHkanno } from '@/services/api/hkanno';
import { OutFormat } from '@/services/api/serde_hkx';
import { hkannoFromFileTab } from '.';

export const FileTabSchema = z.object({
  id: z.string(),
  inputPath: z.string(),
  outputPath: z.string(),
  format: outFormatSchema,
  /** XML index e.g. `#0003`  */
  ptr: z.string(),
  num_original_frames: z.number(),
  duration: z.number(),
  /** Hkanno.AnnotationTrack[] */
  text: z.string(),
  /** file first loaded original hkanno(use on revert). readonly */
  hkanno: HkannoSchema.readonly(),
  dirty: z.boolean().optional(),
});
export type FileTab = z.infer<typeof FileTabSchema>;

/* -------------------------------------------------------------------------- */
/*                               Sub Components                               */
/* -------------------------------------------------------------------------- */

const HeaderToolbar = ({
  onSave,
  onRevert,
  showPreview,
  setShowPreview,
}: {
  onSave: () => void;
  onRevert: () => void;
  showPreview: boolean;
  setShowPreview: (v: boolean) => void;
}) => {
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
};

const FileSettingsBar = ({
  outputPath,
  format,
  onOutputChange,
  onFormatChange,
}: {
  outputPath: string;
  format: OutFormat;
  onOutputChange: (v: string) => void;
  onFormatChange: (v: OutFormat) => void;
}) => {
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
};

const SplitEditors = ({
  tab,
  isVimMode,
  showPreview,
  onTextChange,
}: {
  tab: FileTab;
  isVimMode: boolean;
  showPreview: boolean;
  onTextChange: (v: string) => void;
}) => {
  const [previewXml, setPreviewXml] = useState('');
  const [hasError, setHasError] = useState(false);
  const { registerLeft, registerRight, updateBaseLine } = useMonacoSyncJump();

  React.useEffect(() => {
    if (showPreview) {
      (async () => {
        if (!tab) return;
        try {
          const parsed = hkannoFromFileTab(tab);
          const xml = await previewHkanno(tab.inputPath, parsed);
          setPreviewXml(xml);
          setHasError(false);
          updateBaseLine(tab.text, xml);
        } catch (_err) {
          setHasError(true);
        }
      })();
    }
  }, [showPreview, tab]);

  return (
    <Allotment>
      {/* Left: Annotation editor */}
      <Allotment.Pane minSize={300}>
        <Box sx={{ height: '100%' }}>
          <Typography variant='subtitle2' sx={{ px: 2, pt: 1, color: '#aaa' }}>
            Annotation
          </Typography>
          <MonacoEditor
            height='calc(87% - 24px)'
            defaultLanguage='hkanno' // NOTE: Comments starting with `#` are being used as pseudo-comments.
            value={tab.text}
            onChange={(val) => val && onTextChange(val)}
            options={{
              'semanticHighlighting.enabled': true,
              fontSize: 13,
              minimap: { enabled: true },
              renderWhitespace: 'boundary',
              rulers: [80],
            }}
            vimMode={isVimMode}
            onMount={registerLeft}
          />
        </Box>
      </Allotment.Pane>

      {/* Right: Preview */}
      {showPreview && (
        <Allotment.Pane minSize={200} preferredSize={680}>
          <Box sx={{ height: '100%' }}>
            <Typography
              variant='subtitle2'
              sx={{
                px: 2,
                pt: 1,
                color: hasError ? '#ff5555' : '#aaa',
              }}
            >
              {hasError ? 'Preview (Error occurred)' : 'Preview'}
            </Typography>
            <MonacoEditor
              key='preview-editor'
              height='calc(87% - 24px)'
              defaultLanguage='xml'
              value={previewXml}
              options={{
                fontSize: 13,
                minimap: { enabled: false },
                readOnly: true,
                renderWhitespace: 'boundary',
              }}
              vimMode={isVimMode}
              onMount={registerRight}
            />
          </Box>
        </Allotment.Pane>
      )}
    </Allotment>
  );
};

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
