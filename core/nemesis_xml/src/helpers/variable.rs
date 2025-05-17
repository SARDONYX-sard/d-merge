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
//!              <hkcstring>eventSample</hkcstring> <!-- (index is  0) -->
//!              <hkcstring>eventSample</hkcstring> <!-- (index is  1) -->
//!              <hkcstring>eventSample</hkcstring> <!-- (index is  2) -->
//!              <hkcstring>eventSample</hkcstring> <!-- (index is  3) -->
//!              <hkcstring>eventSample</hkcstring> <!-- (index is  4) -->
//!              <hkcstring>eventSample</hkcstring> <!-- (index is  5) -->
//!              <hkcstring>eventSample</hkcstring> <!-- (index is  6) -->
//!              <hkcstring>eventSample</hkcstring> <!-- (index is  7) -->
//!              <hkcstring>eventSample</hkcstring> <!-- (index is  8) -->
//!              <hkcstring>eventSample</hkcstring> <!-- (index is  9) -->
//! <!-- MOD_CODE ~sample~ OPEN -->
//!              <hkcstring>eventSample</hkcstring> <!-- (index is 10) -->
//! <!-- CLOSE -->
//!          </hkparam>
//!          <hkparam name="variableNames" numelements="9">
//!              <hkcstring>variableSample</hkcstring> <!-- (index is  0) -->
//!              <hkcstring>variableSample</hkcstring> <!-- (index is  1) -->
//!              <hkcstring>variableSample</hkcstring> <!-- (index is  2) -->
//!              <hkcstring>variableSample</hkcstring> <!-- (index is  3) -->
//!              <hkcstring>variableSample</hkcstring> <!-- (index is  4) -->
//!              <hkcstring>variableSample</hkcstring> <!-- (index is  5) -->
//!              <hkcstring>variableSample</hkcstring> <!-- (index is  6) -->
//!              <hkcstring>variableSample</hkcstring> <!-- (index is  7) -->
//!              <hkcstring>variableSample</hkcstring> <!-- (index is  8) -->
//!              <hkcstring>variableSample</hkcstring> <!-- (index is  9) -->
//! <!-- MOD_CODE ~sample~ OPEN -->
//!              <hkcstring>variableSample</hkcstring> <!-- (index is 10) -->
//! <!-- CLOSE -->
//!          </hkparam>
//!      <hkobject>
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
    ascii::Caseless,
    combinator::separated_pair,
    error::{StrContext::*, StrContextValue::*},
    token::take_until,
    ModalResult, Parser,
};

/// # Errors
/// If not found `$eventID[` `]`
pub fn event_id<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    separated_pair(Caseless("$eventID["), take_until(0.., "]$"), "]$")
        .take()
        .context(Expected(Description(
            "eventID(e.g. `$eventID[sampleEventName]$`)",
        )))
        .parse_next(input)
}

/// # Errors
/// If not found `$variableID[` `]`
pub fn variable_id<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    separated_pair(Caseless("$variableID["), take_until(0.., "]$"), "]$")
        .take()
        .context(Expected(Description(
            "variableID(e.g. `$variableID[sampleName]$`)",
        )))
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable() {
        let event_name = event_id.parse_next(&mut "$eventID[sampleEvent]$");
        assert_eq!(event_name, Ok("$eventID[sampleEvent]$"));
        let event_name = event_id.parse_next(&mut "$eventID[sampleEvent]$remain");
        assert_eq!(event_name, Ok("$eventID[sampleEvent]$"));

        let var_name = variable_id.parse_next(&mut "$variableID[sampleVal]$");
        assert_eq!(var_name, Ok("$variableID[sampleVal]$"));
    }
}
