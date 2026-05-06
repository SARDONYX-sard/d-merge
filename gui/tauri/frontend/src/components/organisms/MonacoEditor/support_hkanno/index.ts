import monaco from 'monaco-editor';
import z from 'zod';
import { registerCodeActionProvider } from './providers/code_action';
import { registerCompletionProvider } from './providers/completion';
import { clearDiagnostics, registerCodeLen } from './providers/diagnostic';
import { registerDocumentFormattingEditProvider } from './providers/formatter';
import { registerHoverProvider } from './providers/hover';
import { registerInlayHintsProvider } from './providers/inlay_hint';
import { registerMonarchTokensProvider } from './providers/monarch_token';
import { registerDocumentSemanticTokensProvider } from './providers/semantic_token';
import { registerSignatureHelpProvider } from './providers/signature';

export const HKANNO_LANGUAGE_ID = 'hkanno';

export type HkAnnoLspOptions = z.infer<typeof HkAnnoLspOptionsSchema>;

export const HkAnnoLspOptionsSchema = z.object({
  completion: z.boolean().optional(),
  codeAction: z.boolean().optional(),
  diagnostics: z.boolean().optional(),
  formatter: z.boolean().optional(),
  semanticTokens: z.boolean().optional(),
  hover: z.boolean().optional(),
  inlayHints: z.boolean().optional(),
  signatureHelp: z.boolean().optional(),
});

export const DEFAULT_HKANNO_LSP_OPTIONS = {
  completion: true,
  codeAction: true,
  diagnostics: true,
  formatter: true,
  semanticTokens: true,
  hover: true,
  inlayHints: false,
  signatureHelp: false,
} as const satisfies HkAnnoLspOptions;

export type NewProviderFn = (monacoEnv: typeof monaco) => monaco.IDisposable;

export const supportHkanno = (options: HkAnnoLspOptions = {}) => {
  return (editor: monaco.editor.IStandaloneCodeEditor, monacoEnv: typeof monaco): monaco.IDisposable[] => {
    monacoEnv.languages.register({ id: HKANNO_LANGUAGE_ID });

    const disposables: monaco.IDisposable[] = [];

    if (options.diagnostics !== false) {
      disposables.push(...registerCodeLen(editor, monacoEnv));
    } else {
      clearDiagnostics(monacoEnv);
    }

    if (options.diagnostics !== false) {
      disposables.push(registerCodeActionProvider(monacoEnv));
    }
    if (options.completion !== false) {
      disposables.push(registerCompletionProvider(monacoEnv));
    }

    if (options.formatter !== false) {
      disposables.push(registerDocumentFormattingEditProvider(monacoEnv));
    }

    if (options.semanticTokens !== false) {
      disposables.push(registerDocumentSemanticTokensProvider(monacoEnv));
    }

    if (options.hover !== false) {
      disposables.push(registerHoverProvider(monacoEnv));
    }

    if (options.inlayHints !== false) {
      disposables.push(registerInlayHintsProvider(monacoEnv));
    }

    registerMonarchTokensProvider(monacoEnv); // no dispose

    if (options.signatureHelp !== false) {
      disposables.push(registerSignatureHelpProvider(monacoEnv));
    }

    return disposables;
  };
};
