//! Payload Interpreter (`PIE`) instruction definitions.
//!
//! Shared by completion (snippets), diagnostics (parameter count),
//! signature help and hover.
//!
//! - Mod: <https://www.nexusmods.com/skyrimspecialedition/mods/65089>
//! - Reference: <https://github.com/D7ry/PayloadInterpreter?tab=readme-ov-file#list-of-instructions>

use crate::parser::PiePrefix;

/// One known instruction.
#[derive(Debug, Clone, Copy)]
pub struct PieInstruction {
    /// e.g., `SGVB`
    pub name: &'static str,
    /// Markdown shown by completion / signature help.
    pub documentation: &'static str,
    /// Completion snippet inserted after the prefix (see [`crate::snippet`]).
    pub snippet: &'static str,
}

impl PieInstruction {
    /// Number of `|`-separated parameters the instruction expects.
    ///
    /// Derived from the snippet so the two never get out of sync.
    /// e.g., `SGVB|${1:graphVariable}|${2:bool}` -> 2
    pub fn param_count(&self) -> usize {
        self.snippet.matches("${").count()
    }
}

/// `@` instructions built into Payload Interpreter.
pub const NATIVE_INSTRUCTIONS: &[PieInstruction] = &[
    PieInstruction {
        name: "SGVB",
        documentation: "```hkanno\nPIE.@SGVB|<graphVariable>|<bool>\n```\nSet an animation boolean variable.",
        snippet: "SGVB|${1:graphVariable}|${2:bool}",
    },
    PieInstruction {
        name: "SGVF",
        documentation: "```hkanno\nPIE.@SGVF|<graphVariable>|<float>\n```\nSet an animation float variable.",
        snippet: "SGVF|${1:graphVariable}|${2:float}",
    },
    PieInstruction {
        name: "SGVI",
        documentation: "```hkanno\nPIE.@SGVI|<graphVariable>|<int>\n```\nSet an animation integer variable.",
        snippet: "SGVI|${1:graphVariable}|${2:int}",
    },
    PieInstruction {
        name: "CASTSPELL",
        documentation: "```hkanno\nPIE.@CASTSPELL|<spellID>|<esp>|<effectiveness>|<magnitude>|<selfTargeting>|<HealthReq>|<HealthCost>|<StaminaReq>|<StaminaCost>|<MagickaReq>|<MagickaCost>\n```\nCast a spell on the actor. Spell may stay on actor.",
        snippet: "CASTSPELL|${1:spellID}|${2:esp}|${3:effectiveness}|${4:magnitude}|${5:selfTargeting}|${6:HealthReq}|${7:HealthCost}|${8:StaminaReq}|${9:StaminaCost}|${10:MagickaReq}|${11:MagickaCost}",
    },
    PieInstruction {
        name: "APPLYSPELL",
        documentation: "```hkanno\nPIE.@APPLYSPELL|<spellID>|<esp>\n```\nApply a spell instantly.",
        snippet: "APPLYSPELL|${1:spellID}|${2:esp}",
    },
    PieInstruction {
        name: "UNAPPLYSPELL",
        documentation: "```hkanno\nPIE.@UNAPPLYSPELL|<spellID>|<esp>\n```\nRemove a spell effect.",
        snippet: "UNAPPLYSPELL|${1:spellID}|${2:esp}",
    },
    PieInstruction {
        name: "SETGHOST",
        documentation: "```hkanno\nPIE.@SETGHOST|<bool>\n```\nMake the actor ghost (invincible).",
        snippet: "SETGHOST|${1:bool}",
    },
    PieInstruction {
        name: "PLAYPARTICLE",
        documentation: "```hkanno\nPIE.@PLAYPARTICLE|<nifPath>|<bodyPartIndex>|<scale>|<playTime>|<flags>|<X>|<Y>|<Z>\n```\nPlay a nif particle effect on the actor.",
        snippet: "PLAYPARTICLE|${1:nifPath}|${2:bodyPartIndex}|${3:scale}|${4:playTime}|${5:flags}|${6:X}|${7:Y}|${8:Z}",
    },
];

/// `$` custom instruction example (the real keys are defined by each user).
pub const CUSTOM_INSTRUCTIONS: &[PieInstruction] = &[PieInstruction {
    name: "MyCustom1",
    documentation: "Custom payload mapped via $KEY",
    snippet: "${1:MyCustom1}|${2:param1}|${3:param2}",
}];

/// `!` async instruction example.
pub const ASYNC_INSTRUCTIONS: &[PieInstruction] = &[PieInstruction {
    name: "DelayExample",
    documentation: "Async payload example ![time]payload",
    snippet: "[${1:time}]${2:payload}",
}];

/// Instructions that can follow `prefix`.
pub const fn instructions_for(prefix: PiePrefix) -> &'static [PieInstruction] {
    match prefix {
        PiePrefix::Native => NATIVE_INSTRUCTIONS,
        PiePrefix::Custom => CUSTOM_INSTRUCTIONS,
        PiePrefix::Async => ASYNC_INSTRUCTIONS,
    }
}

/// Finds a native instruction by name (case-insensitive).
///
/// e.g., `find_native("sgvb")` -> `Some(SGVB)`
pub fn find_native(name: &str) -> Option<&'static PieInstruction> {
    NATIVE_INSTRUCTIONS.iter().find(|ins| ins.name.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn param_count_from_snippet() {
        assert_eq!(find_native("sgvb").map(PieInstruction::param_count), Some(2));
        assert_eq!(find_native("CASTSPELL").map(PieInstruction::param_count), Some(11));
    }
}
