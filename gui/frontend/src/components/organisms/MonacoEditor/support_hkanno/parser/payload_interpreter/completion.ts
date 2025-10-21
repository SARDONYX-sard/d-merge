import * as monaco from 'monaco-editor';
import { isAfter } from '../../providers/completion';
import type { PayloadInstructionNode } from './nodes';

export const PIE_NATIVE_INSTRUCTIONS = [
  {
    name: 'SGVB',
    documentation: `\`\`\`hkanno
PIE.@SGVB|<graphVariable>|<bool>
\`\`\`
Set an animation boolean variable.`,
    snippet: 'SGVB|${1:graphVariable}|${2:bool}',
  },
  {
    name: 'SGVF',
    documentation: `\`\`\`hkanno
PIE.@SGVF|<graphVariable>|<float>
\`\`\`
Set an animation float variable.`,
    snippet: 'SGVF|${1:graphVariable}|${2:float}',
  },
  {
    name: 'SGVI',
    documentation: `\`\`\`hkanno
PIE.@SGVI|<graphVariable>|<int>
\`\`\`
Set an animation integer variable.`,
    snippet: 'SGVI|${1:graphVariable}|${2:int}',
  },
  {
    name: 'CASTSPELL',
    documentation: `\`\`\`hkanno
PIE.@CASTSPELL|<spellID>|<esp>|<effectiveness>|<magnitude>|<selfTargeting>|<HealthReq>|<HealthCost>|<StaminaReq>|<StaminaCost>|<MagickaReq>|<MagickaCost>
\`\`\`
Cast a spell on the actor. Spell may stay on actor.`,
    snippet:
      'CASTSPELL|${1:spellID}|${2:esp}|${3:effectiveness}|${4:magnitude}|${5:selfTargeting}|${6:HealthReq}|${7:HealthCost}|${8:StaminaReq}|${9:StaminaCost}|${10:MagickaReq}|${11:MagickaCost}',
  },
  {
    name: 'APPLYSPELL',
    documentation: `\`\`\`hkanno
PIE.@APPLYSPELL|<spellID>|<esp>
\`\`\`
Apply a spell instantly.`,
    snippet: 'APPLYSPELL|${1:spellID}|${2:esp}',
  },
  {
    name: 'UNAPPLYSPELL',
    documentation: `\`\`\`hkanno
PIE.@UNAPPLYSPELL|<spellID>|<esp>
\`\`\`
Remove a spell effect.`,
    snippet: 'UNAPPLYSPELL|${1:spellID}|${2:esp}',
  },
  {
    name: 'SETGHOST',
    documentation: `\`\`\`hkanno
PIE.@SETGHOST|<bool>
\`\`\`
Make the actor ghost (invincible).`,
    snippet: 'SETGHOST|${1:bool}',
  },
  {
    name: 'PLAYPARTICLE',
    documentation: `\`\`\`hkanno
PIE.@PLAYPARTICLE|<nifPath>|<bodyPartIndex>|<scale>|<playTime>|<flags>|<X>|<Y>|<Z>
\`\`\`
Play a nif particle effect on the actor.`,
    snippet: 'PLAYPARTICLE|${1:nifPath}|${2:bodyPartIndex}|${3:scale}|${4:playTime}|${5:flags}|${6:X}|${7:Y}|${8:Z}',
  },
] as const;

export const PIE_CUSTOM_INSTRUCTIONS = [
  {
    name: 'MyCustom1',
    documentation: 'Custom payload mapped via $KEY',
    snippet: '${1:MyCustom1}|${2:param1}|${3:param2}',
  },
] as const;

export const PIE_ASYNC_INSTRUCTIONS = [
  {
    name: 'DelayExample',
    documentation: 'Async payload example ![time]payload',
    snippet: '[${1:time}]${2:payload}',
  },
] as const;

export const providePieCompletions = (
  node: PayloadInstructionNode,
  range: monaco.IRange,
): monaco.languages.CompletionItem[] => {
  if (node.event?.value?.toLocaleLowerCase() !== 'pie') return [];
  if (!node.dot && isAfter(node.event?.pos, range)) {
    return [{ label: '.', kind: monaco.languages.CompletionItemKind.Enum, insertText: '.', range }];
  }

  const prefix = node.instruction?.prefix?.value;
  if (prefix === undefined && isAfter(node.dot?.pos, range)) {
    return prefixCompletions(range);
  }

  if (node.instruction?.name === undefined && isAfter(node.instruction?.prefix?.pos, range)) {
    let instructions;
    switch (prefix) {
      case '@':
        instructions = PIE_NATIVE_INSTRUCTIONS;
        break;
      case '$':
        instructions = PIE_CUSTOM_INSTRUCTIONS;
        break;
      case '!':
        instructions = PIE_ASYNC_INSTRUCTIONS;
        break;
      default:
        return [];
    }

    return instructions.map(({ name, snippet, documentation }) => {
      return {
        label: name,
        kind: monaco.languages.CompletionItemKind.Function,
        insertText: snippet,
        insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
        range,
        documentation: { value: documentation, isTrusted: true },
      };
    });
  }

  return [];
};

const prefixCompletions = (range: monaco.IRange) => [
  {
    label: '@',
    kind: monaco.languages.CompletionItemKind.EnumMember,
    insertText: '@',
    range,
    documentation: { value: 'Native Instruction prefix' },
  },
  {
    label: '$',
    kind: monaco.languages.CompletionItemKind.EnumMember,
    insertText: '$',
    range,
    documentation: { value: 'Custom Instruction prefix' },
  },
  {
    label: '!',
    kind: monaco.languages.CompletionItemKind.EnumMember,
    insertText: '!',
    range,
    documentation: { value: 'Async payload prefix' },
  },
];
