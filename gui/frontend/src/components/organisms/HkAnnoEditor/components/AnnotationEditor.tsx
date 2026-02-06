import { OnChange, OnMount } from '@monaco-editor/react';
import { Typography } from '@mui/material';
import { editor } from 'monaco-editor';
import { useTranslation } from '@/components/hooks/useTranslation';
import { MonacoEditor } from '@/components/organisms/MonacoEditor';
import { useEditorModeContext } from '../../../providers/EditorModeProvider';
import { useEditorContext } from '../context/editorContext';

/** Annotation editor pane */
export const AnnotationEditor = ({ onMount }: { onMount: OnMount }) => {
  const [state, dispatch] = useEditorContext();
  const isVimMode = useEditorModeContext().editorMode === 'vim';
  const tab = state.tabs[state.active];
  const { t } = useTranslation();

  const handleOnChange: OnChange = (text) => {
    if (text) {
      dispatch({ type: 'UPDATE_TEXT', text });
    }
  };

  const handleOnMount: OnMount = (editor, monaco) => {
    onMount(editor, monaco);

    // Restore cursorPos in FileTab
    if (tab.cursorPos) {
      editor.setPosition(tab.cursorPos);
      editor.revealPositionInCenter(tab.cursorPos);
      editor.focus();
    }

    // Save position
    editor.onDidChangeCursorPosition(() => {
      const pos = editor.getPosition();
      if (pos) {
        dispatch({
          type: 'UPDATE_CURSOR',
          cursorPos: pos,
        });
      }
    });
  };

  return (
    <>
      <Typography variant='subtitle2' sx={{ px: 2, pt: 1, color: '#aaa' }}>
        {t('hkanno.editor.annotation')}
      </Typography>
      <MonacoEditor
        key={tab.id + tab.dirty}
        defaultLanguage='hkanno'
        height='90%'
        value={tab.text}
        vimMode={isVimMode}
        options={MONACO_OPTIONS}
        onChange={handleOnChange}
        onMount={handleOnMount}
      />
    </>
  );
};

const MONACO_OPTIONS: editor.IStandaloneEditorConstructionOptions = {
  'semanticHighlighting.enabled': true,
  fontSize: 13,
  minimap: { enabled: true },
  renderWhitespace: 'boundary',
  bracketPairColorization: {
    enabled: true,
  },
};
