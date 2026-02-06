import { OnMount } from '@monaco-editor/react';
import { Typography } from '@mui/material';
import { useEffect, useState } from 'react';
import { useTranslation } from '@/components/hooks/useTranslation';
import { MonacoEditor } from '@/components/organisms/MonacoEditor';
import { Hkanno, hkannoFromText } from '@/services/api/hkanno';
import { previewHkanno } from '../../../../services/api/hkanno';
import { useEditorContext } from '../context/editorContext';
import { FileTab } from '../types/FileTab';

export const PreviewEditor = ({
  onMount,
  updateBaseLine,
}: {
  onMount: OnMount;
  updateBaseLine: (left: string, right: string) => void;
}) => {
  const { t } = useTranslation();

  const { previewXml, error } = usePreviewXml(updateBaseLine);

  return (
    <>
      <Typography variant='subtitle2' sx={{ px: 2, pt: 1, color: error ? '#ff5555' : '#aaa' }}>
        {error ? `${t('hkanno.preview.error_title')}: ${error}` : t('hkanno.preview.title')}
      </Typography>
      <MonacoEditor
        key='preview-editor'
        height='90%'
        defaultLanguage='xml'
        value={previewXml}
        options={{
          fontSize: 13,
          minimap: { enabled: false },
          readOnly: true,
          renderWhitespace: 'boundary',
        }}
        // vimMode={isVimMode} // NOTE: When multiple Vim mode editors are open simultaneously, commands like `hover` mysteriously stop working.
        onMount={onMount}
      />
    </>
  );
};

const usePreviewXml = (updateBaseLine: (left: string, right: string) => void) => {
  const [state, _dispatch] = useEditorContext();
  const tab = state.tabs[state.active];
  const [previewXml, setPreviewXml] = useState('');
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!tab || !state.showPreview) return;

    (async () => {
      try {
        const parsed = hkannoFromFileTab(tab);
        const xml = await previewHkanno(tab.inputPath, parsed);
        setPreviewXml(xml);
        updateBaseLine(tab.text, xml);
        setError(null);
      } catch (e) {
        setError(String(e));
        setPreviewXml('');
      }
    })();
  }, [tab, state.showPreview]);

  return { previewXml, error };
};

/** Convert editor tab into Hkanno object */
const hkannoFromFileTab = (tab: FileTab): Hkanno => ({
  ptr: tab.ptr,
  num_original_frames: tab.num_original_frames,
  duration: tab.duration,
  annotation_tracks: hkannoFromText(tab.text),
});
