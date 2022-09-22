use serde::{Deserialize, Serialize};

use crate::util::ValueMatcher;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    strum_macros::Display,
    strum_macros::EnumString,
)]
pub enum HypMethod {
    Assign,
    Host,
    Provision,
    Knock,
    Hop,
    Transport,
    HyperWave,
    Search,
}

impl ValueMatcher<HypMethod> for HypMethod {
    fn is_match(&self, x: &HypMethod) -> Result<(), ()> {
        if *x == *self {
            Ok(())
        } else {
            Err(())
        }
    }
}
