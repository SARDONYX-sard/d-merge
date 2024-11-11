// SPDX-FileCopyrightText: (C) 2018 Cisco Guillaume
// SPDX-License-Identifier: MIT
//
// js to tsx
// ref: https://github.com/GuillaumeCisco/react-json-prettify
// ver: 0.2.0
import { def } from './themes/atomOneDark';

import type React from 'react';

type JsonPrimitive = string | number | boolean | null | undefined;
type JsonValue = Json[] | JsonPrimitive;
type Json = Record<string, JsonValue> | JsonValue;

// Define types for the theme
interface Theme {
  background: string;
  value: {
    null: string | ((value: null) => string);
    string: string | ((value: string) => string);
    number: string | ((value: number) => string);
    boolean: string | ((value: boolean) => string);
  };
  valueQuotes: string | ((value: Json) => string);
  bracket: string;
  brace: string;
  keyQuotes: string;
  key: string;
  colon: string;
  comma: string;
}

interface RecursiveKeyValueProps {
  parent: string;
  value: Json;
  theme: Theme;
  padding: number;
  deep: number;
}

const RecursiveKeyValueDefaultProps = {
  parent: '',
  value: undefined,
  theme: def,
  padding: 2,
  deep: 0,
} as const satisfies RecursiveKeyValueProps;

const renderNull = (theme: Theme) => (
  <span style={{ color: typeof theme.value.null === 'function' ? theme.value.null(null) : theme.value.null }}>
    null
  </span>
);

const renderString = (value: string, theme: Theme) => (
  <span style={{ color: typeof theme.valueQuotes === 'function' ? theme.valueQuotes(value) : theme.valueQuotes }}>
    &quot;
    <span style={{ color: typeof theme.value.string === 'function' ? theme.value.string(value) : theme.value.string }}>
      {value}
    </span>
    &quot;
  </span>
);

const renderNumber = (value: number, theme: Theme) => (
  <span style={{ color: typeof theme.value.number === 'function' ? theme.value.number(value) : theme.value.number }}>
    {value}
  </span>
);

const renderBoolean = (value: boolean, theme: Theme) => (
  <span style={{ color: typeof theme.value.boolean === 'function' ? theme.value.boolean(value) : theme.value.boolean }}>
    {value ? 'true' : 'false'}
  </span>
);

const renderArray = (parent: string, value: Json[], theme: Theme, padding: number, deep: number) => {
  const valueQuotes = typeof theme.valueQuotes === 'function' ? '' : theme.valueQuotes;

  return (
    <>
      <span style={{ color: theme.bracket }}>{'['}</span>
      <div>
        {value.map((item, index) => (
          <div key={`${parent}-${item}`} style={{ color: valueQuotes }}>
            {new Array(deep * padding + 1).join('\u00A0')}
            <RecursiveKeyValue deep={deep} padding={padding} parent={parent} theme={theme} value={item} />
            {index === value.length - 1 ? '' : <span style={{ color: theme.comma }}>,</span>}
          </div>
        ))}
      </div>
      <span style={{ color: theme.bracket }}>{new Array((deep - 1) * padding + 1).join('\u00A0')}]</span>
    </>
  );
};

const renderObject = (
  parent: string,
  value: Record<string, JsonValue>,
  theme: Theme,
  padding: number,
  deep: number,
) => {
  const keys = Object.keys(value);
  return (
    <>
      <span style={{ color: theme.brace }}>{'{'}</span>
      <div>
        {keys.map((key, index) => (
          <div key={`${parent}-${key}-${index}-${deep}`}>
            <span>{new Array(deep * padding + 1).join('\u00A0')}</span>
            <span style={{ color: theme.keyQuotes }}>
              &quot;<span style={{ color: theme.key }}>{key}</span>&quot;<span style={{ color: theme.colon }}>: </span>
            </span>
            <RecursiveKeyValue deep={deep} padding={padding} parent={key} theme={theme} value={value[key]} />
            {index === keys.length - 1 ? '' : <span style={{ color: theme.comma }}>,</span>}
          </div>
        ))}
      </div>
      <span style={{ color: theme.brace }}>{new Array((deep - 1) * padding + 1).join('\u00A0')}</span>
    </>
  );
};

const RecursiveKeyValue: React.FC<RecursiveKeyValueProps> = ({
  parent,
  value,
  theme,
  padding,
  deep,
} = RecursiveKeyValueDefaultProps) => {
  deep += 1;

  if (value === null) {
    return renderNull(theme);
  }
  if (Array.isArray(value)) {
    return renderArray(parent, value, theme, padding, deep);
  }

  switch (typeof value) {
    case 'string':
      return renderString(value, theme);
    case 'number':
      return renderNumber(value, theme);
    case 'boolean':
      return renderBoolean(value, theme);
    case 'object':
      return renderObject(parent, value, theme, padding, deep);
    default:
      return value;
  }
};

export interface JsonPrettyProps {
  json: Json;
  theme?: Theme;
  padding?: number;
}

const defaultProps = {
  json: null,
  theme: def,
  padding: 2,
};

export const JsonPretty: React.FC<JsonPrettyProps> = (props) => {
  const { json, theme, padding } = { ...defaultProps, ...props };

  return (
    <pre style={{ overflow: 'auto', backgroundColor: theme.background }}>
      <RecursiveKeyValue deep={0} padding={padding} parent='root' theme={theme} value={json} />
    </pre>
  );
};
