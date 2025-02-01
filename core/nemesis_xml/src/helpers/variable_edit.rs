//! # Nemesis variables
//! - Nemesis abstracts the index of the eventName inserted in `eventNames`(or `variableNames`) of `hkbBehaviorGraphStringData` and makes it a variable since it may come here.
//! 1. in the creation stage of this patch, it is necessary to replace eventName with index, so it is reserved as a string here. 2.
//! 2. replace them in the merge stage of patch.
//!
//! # Example
//! `<! -- (omit) -->` just omits the XML.
//!
//! - #0052.txt
//! ```xml
//!      <hkobject name="#0052" class="hkbBehaviorGraphStringData" signature="0xc713064e">
//!          <hkparam name="eventNames" numelements="9">
//!              <hkcstring>staggerStop</hkcstring>
//!              <!-- (omit) -->
//! <!-- MOD_CODE ~sample~ OPEN -->
//!              <hkcstring>eventSample</hkcstring> <!-- (index is 10) -->
//! <!-- CLOSE -->
//!          </hkparam>
//!          <hkparam name="variableNames" numelements="9">
//!              <!-- (omit) -->
//! <!-- MOD_CODE ~sample~ OPEN -->
//!              <hkcstring>variableSample</hkcstring> <!-- (index is 10) -->
//! <!-- CLOSE -->
//!          </hkparam>
//! ```
//!
//! - #sample$1.txt
//! ```xml
//! <hkobject name="#sample$1" class="hkbVariableBindingSet" signature="0x338ad4ff">
//!     <hkparam name="bindings" numelements="1">
//!         <hkobject>
//!              <!-- (omit) -->
//!             <hkparam name="variableIndex">$variableID[variableSample]$<!-- (== Replace with 10) --></hkparam>
//!             <hkparam name="bitIndex">-1</hkparam>
//!             <hkparam name="bindingType">BINDING_TYPE_VARIABLE</hkparam>
//!         </hkobject>
//!     </hkparam>
//!     <!-- (omit) -->
//! </hkobject>
//! ```
use winnow::{
    ascii::Caseless, combinator::delimited, error::ContextError, token::take_until, PResult, Parser,
};

// プレースホルダの例として使用するイベント名と変数名のリスト
const EVENT_NAMES: [&str; 2] = ["staggerStop", "eventSample"]; // 例: イベント名
const VARIABLE_NAMES: [&str; 1] = ["variableSample"]; // 例: 変数名

/// eventID のプレースホルダをインデックスに置き換える関数
pub fn event_id<'a>(input: &mut &'a str, event_names: &[&str]) -> PResult<String> {
    let event_name =
        delimited(Caseless("$eventID["), take_until(0.., "]$"), "]$").parse_next(input)?;

    if let Some(index) = event_names.iter().position(|&name| name == event_name) {
        Ok(index.to_string())
    } else {
        Err(ContextError::new())
        // Err(input.error(Expected(Description("Valid event name in eventNames"))))
    }
}

/// variableID のプレースホルダをインデックスに置き換える関数
pub fn variable_id<'a>(input: &mut &'a str, variable_names: &[&str]) -> PResult<String> {
    let variable_name =
        delimited(Caseless("$variableID["), take_until(0.., "]$"), "]$").parse_next(input)?;

    if let Some(index) = variable_names
        .iter()
        .position(|&name| name == variable_name)
    {
        Ok(index.to_string())
    } else {
        // Err(input.(Expected(Description(
        //     "Valid variable name in variableNames",
        // ))))
        let text: &str = "error";
        Err(ContextError::new(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_event_id() {
        let mut input = "$eventID[eventSample]$";
        let result = event_id(&mut input, &EVENT_NAMES);
        assert_eq!(result, Ok("1".to_string())); // eventSampleはインデックス1
    }

    #[test]
    fn test_replace_variable_id() {
        let mut input = "$variableID[variableSample]$";
        let result = variable_id(&mut input, &VARIABLE_NAMES);
        assert_eq!(result, Ok("0".to_string())); // variableSampleはインデックス0
    }
}
