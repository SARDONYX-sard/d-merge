// SPDX-FileCopyrightText: (C) 2018 Cisco Guillaume
// SPDX-License-Identifier: MIT
//
// js to tsx
// ref: https://github.com/GuillaumeCisco/react-json-prettify
// ver: 0.2.0
export const def = {
  background: 'rgb(40, 44, 52)',
  brace: 'rgb(171, 178, 191)',
  keyQuotes: 'rgb(209, 154, 102)',
  valueQuotes: 'rgb(152, 195, 121)',
  colon: 'rgb(171, 178, 191)',
  comma: 'rgb(171, 178, 191)',
  key: 'rgb(209, 154, 102)',
  value: {
    string: 'rgb(152, 195, 121)',
    null: 'rgb(86, 182, 194)',
    number: 'rgb(209, 154, 102)',
    boolean: 'rgb(86, 182, 194)',
  },
  bracket: 'rgb(171, 178, 191)',
} as const;
