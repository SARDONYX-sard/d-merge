import { OnMount } from '@monaco-editor/react';
import { useEffect, useRef } from 'react';
import { MonacoEditor } from '@/components/organisms/MonacoEditor';
import { HkAnnoLspOptions, supportHkanno } from '@/components/organisms/MonacoEditor/support_hkanno';

import type monaco from 'monaco-editor';

export const useHkAnnoLsp = (options: HkAnnoLspOptions) => {
  const editorRef = useRef<MonacoEditor | null>(null);
  const monacoRef = useRef<typeof monaco | null>(null);
  const disposablesRef = useRef<monaco.IDisposable[]>([]);

  // NOTE: Prevent duplicate registrations when switching editor tabs
  useEffect(() => {
    if (!editorRef.current || !monacoRef.current) return;

    disposablesRef.current.forEach((d) => d.dispose());
    disposablesRef.current = [];
    disposablesRef.current = supportHkanno(options)(editorRef.current, monacoRef.current);
  }, [options]);

  // NOTE: Prevent duplicate registrations when navigating away and returning to the page
  useEffect(() => {
    return () => {
      if (disposablesRef.current) {
        disposablesRef.current.forEach((d) => d.dispose());
        disposablesRef.current = [];
      }
    };
  }, []);

  const handleOnMount: OnMount = (editor, monaco) => {
    editorRef.current = editor;
    monacoRef.current = monaco;

    // NOTE: To avoid duplicate registration of language features, dispose existing ones before registering new ones.
    if (disposablesRef.current) {
      disposablesRef.current.forEach((d) => d.dispose());
      disposablesRef.current = [];
    }
    disposablesRef.current = supportHkanno(options)(editor, monaco);
  };

  return handleOnMount;
};
