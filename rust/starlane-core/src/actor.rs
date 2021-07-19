use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::str::FromStr;
use std::sync::Arc;

use serde::{Deserialize, Serialize, Serializer};
use serde::de::DeserializeOwned;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::sync::broadcast::Sender;

use crate::app::ConfigSrc;
use crate::error::Error;
use crate::frame::Event;
use crate::id::Id;
use crate::message::Fail;
use crate::names::Name;
use crate::resource::{AppKey, Labels, Names, ResourceAddress, ResourceAddressPart, ResourceArchetype, ResourceAssign, ResourceCreate, ResourceKind, ResourceRecord, ResourceRegistration, ResourceRegistryInfo, ResourceSelector, ResourceStub};
use crate::star::StarKey;

pub type ActorSpecific = Name;
pub type GatheringSpecific = Name;

#[derive(Debug,Eq, PartialEq, Hash, Clone, Serialize, Deserialize)]
pub enum ActorKind {
    Stateful,
    Stateless,
}

impl ActorKind {
    // it looks a little pointless but helps get around a compiler problem with static_lazy values
    pub fn as_kind(&self) -> Self {
        self.clone()
    }
}

impl fmt::Display for ActorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ActorKind::Stateful => "Stateful".to_string(),
                ActorKind::Stateless => "Stateless".to_string(),
            }
        )
    }
}

impl FromStr for ActorKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Stateful" => Ok(ActorKind::Stateful),
            "Stateless" => Ok(ActorKind::Stateless),
            _ => Err(format!("could not find ActorKind: {}", s).into()),
        }
    }
}
