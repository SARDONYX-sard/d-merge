import * as monaco from 'monaco-editor';

// -------------------- Types --------------------
export type MetaInfo = {
  numOriginalFrames?: number;
  duration?: number;
  numAnnotationTracks?: number;
  numAnnotations?: number[];
};

export type Annotation = { time: number; text: string };
export type Track = { annotations: Annotation[] };

export type Validation = {
  lineNumber: number;
  message: string;
  severity: 'error' | 'warning';
};

// -------------------- Parsing Functions --------------------
export const parseMetaLine = (line: string): Partial<MetaInfo> | null => {
  const match = /^# (\w+): (.+)/.exec(line.trim());
  if (!match) return null;
  const [_, key, value] = match;
  switch (key) {
    case 'numOriginalFrames':
      return { numOriginalFrames: parseInt(value) };
    case 'duration':
      return { duration: parseFloat(value) };
    case 'numAnnotationTracks':
      return { numAnnotationTracks: parseInt(value) };
    default:
      return null;
  }
};

export const parseTrackHeader = (line: string) => /^# numAnnotations: (\d+)/.test(line.trim());

export const parseAnnotationLine = (
  line: string,
  lineNumber: number,
): { annotation?: Annotation; validation?: Validation } => {
  const trimmed = line.trimStart();
  if (!trimmed || trimmed.startsWith('#')) return { annotation: undefined, validation: undefined };

  const parts = trimmed.split(/\s+/);
  if (parts.length < 2) {
    return { validation: { lineNumber, message: 'Annotation line must contain <time> <text>', severity: 'error' } };
  }

  const time = parseFloat(parts[0]);
  if (isNaN(time)) {
    return { validation: { lineNumber, message: 'Time must be a valid number', severity: 'error' } };
  }

  return { annotation: { time, text: parts.slice(1).join(' ') }, validation: undefined };
};

export const parseHkannoText = (text: string) => {
  const lines = text.split('\n');
  const meta: MetaInfo = {};
  const tracks: Track[] = [];
  const validations: Validation[] = [];
  let currentTrack: Track | null = null;

  lines.forEach((line, i) => {
    const lineNumber = i + 1;
    const metaLine = parseMetaLine(line);
    if (metaLine) {
      Object.assign(meta, metaLine);
      return;
    }

    if (parseTrackHeader(line)) {
      currentTrack = { annotations: [] };
      tracks.push(currentTrack);
      return;
    }

    const { annotation, validation } = parseAnnotationLine(line, lineNumber);
    if (annotation && currentTrack) currentTrack.annotations.push(annotation);
    if (validation) validations.push(validation);
  });

  return { meta, tracks, validations };
};

// -------------------- Formatter --------------------
export const formatHkannoText = (text: string): string => {
  const lines = text.split('\n');
  const formatted: string[] = [];
  for (const line of lines) {
    const trimmed = line.trimStart();
    if (trimmed.startsWith('#')) {
      formatted.push(trimmed);
      continue;
    }
    const parts = trimmed.split(/\s+/);
    if (parts.length >= 2) {
      formatted.push(`${parts[0]} ${parts.slice(1).join(' ')}`);
    } else {
      formatted.push(trimmed);
    }
  }
  return formatted.join('\n');
};

// -------------------- Semantic Tokens --------------------
export class SimpleSemanticTokensBuilder {
  private _data: number[] = [];
  private _prevLine = 0;
  private _prevChar = 0;
  private _tokenTypes: Record<string, number>;

  constructor(tokenTypes: string[]) {
    this._tokenTypes = Object.fromEntries(tokenTypes.map((t, i) => [t, i]));
  }

  push(line: number, start: number, length: number, tokenType: string, tokenModifiers = 0) {
    const deltaLine = line - this._prevLine;
    const deltaStart = deltaLine === 0 ? start - this._prevChar : start;
    this._data.push(deltaLine, deltaStart, length, this._tokenTypes[tokenType], tokenModifiers);
    this._prevLine = line;
    this._prevChar = start;
  }

  build() {
    return { data: new Uint32Array(this._data) };
  }
}

export const getHkannoTokensForLine = (line: string, lineIndex: number, tokenTypes: string[]) => {
  const builder = new SimpleSemanticTokensBuilder(tokenTypes);
  const trimmed = line.trim();
  if (trimmed.startsWith('#')) {
    builder.push(lineIndex, line.indexOf('#'), line.length, 'comment');
    return builder.build();
  }
  const metaMatch = /^# (\w+):/.exec(trimmed);
  if (metaMatch) {
    builder.push(lineIndex, line.indexOf(metaMatch[0]), metaMatch[0].length, 'keyword');
    return builder.build();
  }
  const parts = trimmed.split(/\s+/);
  if (parts.length >= 2) {
    builder.push(lineIndex, line.indexOf(parts[0]), parts[0].length, 'number'); // time
    const textStart = line.indexOf(parts.slice(1).join(' '));
    builder.push(lineIndex, textStart, parts.slice(1).join(' ').length, 'string'); // text
  }
  return builder.build();
};

// ------------------- Completions --------------------
export const parseAnnotationLineSimple = (line: string): number | null => {
  const trimmed = line.trim();

  const firstSpaceIndex = trimmed.indexOf(' ');
  if (firstSpaceIndex < 0) return null; // no space → only time?

  const time = parseFloat(trimmed.slice(0, firstSpaceIndex));
  if (Number.isNaN(time)) return null;

  return time;
};

// -------------------- Diagnostics --------------------
export const applyHkannoDiagnostics = (editor: monaco.editor.IStandaloneCodeEditor, validations: Validation[]) => {
  const model = editor.getModel();
  if (!model) return;

  const markers: monaco.editor.IMarkerData[] = validations.map((v) => ({
    startLineNumber: v.lineNumber,
    endLineNumber: v.lineNumber,
    startColumn: 1,
    endColumn: model.getLineMaxColumn(v.lineNumber),
    message: v.message,
    severity: v.severity === 'error' ? monaco.MarkerSeverity.Error : monaco.MarkerSeverity.Warning,
  }));

  monaco.editor.setModelMarkers(model, 'hkanno', markers);
};

// -------------------- Monaco Registration --------------------
export const supportHkanno = (monacoEnv: typeof monaco) => {
  const tokenTypes = ['keyword', 'number', 'string', 'comment', 'type'];

  monacoEnv.languages.register({ id: 'hkanno' });
  monacoEnv.languages.setLanguageConfiguration('hkanno', {
    comments: {
      lineComment: '#',
    },
  });

  // Monarch fallback tokenizer
  monacoEnv.languages.setMonarchTokensProvider('hkanno', {
    tokenizer: {
      root: [
        [/#.*/, 'comment'],
        [/\d+\.\d+/, 'number.float'],
        [/\d+/, 'number'],
        [/".*?"/, 'string'],
        [/[a-zA-Z0-9_]+:/, 'keyword'],
      ],
    },
  });

  // Completion
  monacoEnv.languages.registerCompletionItemProvider('hkanno', {
    provideCompletionItems(model, position) {
      const lineContent = model.getLineContent(position.lineNumber);
      const range = {
        startLineNumber: position.lineNumber,
        endLineNumber: position.lineNumber,
        startColumn: 1,
        endColumn: lineContent.length + 1,
      };

      const suggestions: monaco.languages.CompletionItem[] = [];

      // annotation line parsing
      if (parseAnnotationLineSimple(lineContent) !== null) {
        // Cursor is after time and there is space → suggest <text>
        suggestions.push({
          label: '<text>',
          kind: monaco.languages.CompletionItemKind.Text,
          insertText: 'Annotation text',
          range,
          documentation: 'Annotation text for this annotation line',
        });
      }

      // General meta completions
      const trimmed = lineContent.trim();
      if (trimmed === '' || trimmed.startsWith('#')) {
        suggestions.push(
          {
            label: '# numAnnotations:',
            kind: monaco.languages.CompletionItemKind.Keyword,
            insertText: '# numAnnotations: ',
            documentation: 'Annotations in track',
            range,
          },
          {
            label: '<time>',
            kind: monaco.languages.CompletionItemKind.Value,
            insertText: '0.0',
            range,
            documentation: 'Time value for this annotation',
          },
        );
      }

      return { suggestions };
    },
  });

  // Hover
  monacoEnv.languages.registerHoverProvider('hkanno', {
    provideHover(model, position) {
      const word = model.getWordAtPosition(position);
      if (!word) return null;
      const lineContent = model.getLineContent(position.lineNumber).trim();
      const hoverMap: Record<string, string> = {
        numOriginalFrames: 'Total frames',
        duration: 'Animation duration',
        numAnnotationTracks: 'Total tracks',
      };
      if (hoverMap[word.word]) return { contents: [{ value: `**${word.word}**: ${hoverMap[word.word]}` }] };

      const parts = lineContent.split(' ');
      if (parts.length >= 2) {
        if (position.column <= parts[0].length + 1) return { contents: [{ value: `**Time**: ${parts[0]}` }] };

        const text = parts.slice(1).join(' ');
        const trimmed = text.trim();
        const isAmr = trimmed.startsWith('animmotion') || trimmed.startsWith('animrotation');
        if (isAmr) {
          return { contents: [{ value: `**AMR annotation Text**: ${text}` }] };
        } else {
          return { contents: [{ value: `**Text**: ${text}` }] };
        }
      }

      return null;
    },
  });

  // Formatter
  monacoEnv.languages.registerDocumentFormattingEditProvider('hkanno', {
    provideDocumentFormattingEdits(model) {
      return [{ range: model.getFullModelRange(), text: formatHkannoText(model.getValue()) }];
    },
  });

  // Semantic Tokens
  monacoEnv.languages.registerDocumentSemanticTokensProvider('hkanno', {
    getLegend() {
      return { tokenTypes, tokenModifiers: [] };
    },
    provideDocumentSemanticTokens(model) {
      const builder = new SimpleSemanticTokensBuilder(tokenTypes);
      model.getLinesContent().forEach((line, i) => {
        const tokens = getHkannoTokensForLine(line, i, tokenTypes);
        const dataArray = tokens.data;
        for (let j = 0; j < dataArray.length; j += 5) {
          builder.push(
            dataArray[j] + builder['_prevLine'],
            dataArray[j + 1] + (dataArray[j] === 0 ? builder['_prevChar'] : 0),
            dataArray[j + 2],
            tokenTypes[dataArray[j + 3]],
            dataArray[j + 4],
          );
        }
      });
      return builder.build();
    },
    releaseDocumentSemanticTokens(_) {},
  });
};
