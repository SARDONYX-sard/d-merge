import * as monaco from 'monaco-editor';
import type { PayloadInstructionNode } from './nodes';

export const PIE_INSTRUCTIONS = [
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

export const providePieCompletions = (
  node: PayloadInstructionNode,
  range: monaco.IRange,
): monaco.languages.CompletionItem[] => {
  if (node.event?.value?.toLocaleLowerCase() !== 'pie') return [];
  if (node.dot?.value !== '.') return [];
  if (node.instruction?.atSymbol === undefined) return [];

  return PIE_INSTRUCTIONS.map((ins) => {
    return {
      label: ins.name,
      kind: monaco.languages.CompletionItemKind.Function,
      insertText: ins.snippet,
      insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
      range,
      documentation: { value: ins.documentation, isTrusted: true },
    };
  });
};
