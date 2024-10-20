import { InputLabel } from '@mui/material';

import { useEditorModeContext } from '@/components/providers/EditorModeProvider';

import { MonacoEditorWrapper } from './MonacoEditorWrapper';

import type { ComponentPropsWithoutRef } from 'react';

export type EditorInfo = {
  css: Props;
  javascript: Props;
};

/** https://github.com/suren-atoyan/monaco-react?tab=readme-ov-file#multi-model-editor */
type Props = {
  value: string;
  /** NOTE: If this is not changed, it is considered the same file change and the change history will be mixed. */
  fileName: string;
  label: string;
  language: string;
  onChange: ComponentPropsWithoutRef<typeof MonacoEditorWrapper>['onChange'];
};

export const EditorInitializer = ({ value, fileName, label, language, onChange }: Props) => {
  const { editorMode } = useEditorModeContext();

  return (
    <>
      <InputLabel sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center', marginTop: '20px' }}>
        {label}
      </InputLabel>
      <MonacoEditorWrapper
        height='500px'
        language={language}
        onChange={onChange}
        options={{
          renderWhitespace: 'boundary',
          rulers: [120],
          hover: { above: true },
        }}
        path={fileName}
        theme='vs-dark'
        value={value}
        vimMode={editorMode === 'vim'}
        width='95%'
      />
      <InputLabel id='status-node' sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center' }} />
    </>
  );
};
