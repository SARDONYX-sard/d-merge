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
    ascii::Caseless,
    combinator::delimited,
    error::{StrContext::*, StrContextValue::*},
    token::take_until,
    PResult, Parser,
};

pub fn event_id<'a>(input: &mut &'a str) -> PResult<&'a str> {
    delimited(Caseless("$eventID["), take_until(0.., "]$"), "]$")
        .context(Expected(Description(
            "eventID(e.g. `$eventID[sampleEventName]$`)",
        )))
        .parse_next(input)
}

pub fn variable_id<'a>(input: &mut &'a str) -> PResult<&'a str> {
    delimited(Caseless("$variableID["), take_until(0.., "]$"), "]$")
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
        assert_eq!(event_name, Ok("sampleEvent"));

        let var_name = variable_id.parse_next(&mut "$variableID[sampleVal]$");
        assert_eq!(var_name, Ok("sampleVal"));
    }
}
