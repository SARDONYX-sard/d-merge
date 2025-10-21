import * as monaco from 'monaco-editor';
import { PIE_NATIVE_INSTRUCTIONS } from './completion';
import type { PayloadInstructionNode } from './nodes';

export const providePieSignatureHelp = (node: PayloadInstructionNode): monaco.languages.SignatureHelpResult => {
  if (!node.instruction?.name?.value) return None();

  const instructionName = node.instruction.name.value.toUpperCase();
  const insDef = PIE_NATIVE_INSTRUCTIONS.find((i) => i.name.toUpperCase() === instructionName);
  if (!insDef) return None();

  const paramCount = node.instruction.parameters?.items.length ?? 0;
  const params =
    node.instruction.parameters?.items.map((p, idx) => ({
      label: `param${idx + 1}`,
      documentation: p.value?.value ?? '',
    })) ?? [];

  return {
    value: {
      signatures: [
        {
          label: insDef.name + '()',
          documentation: insDef.documentation,
          parameters: params,
        },
      ],
      activeSignature: 0,
      activeParameter: Math.min(paramCount, params.length - 1),
    },
    dispose() {},
  };
};

const None = (): monaco.languages.SignatureHelpResult => ({
  value: { signatures: [], activeSignature: 0, activeParameter: 0 },
  dispose() {},
});
