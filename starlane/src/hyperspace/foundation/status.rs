use serde::{Deserialize, Serialize};
use crate::hyperspace::foundation::err::{ActionItem, ActionRequest};
use crate::hyperspace::foundation::kind::{DependencyKind, FoundationKind, Kind};
use crate::space::parse::CamelCase;

/// [`Phase`] (stage,step) ... signifies where a foundation item is in it's provisioning process
#[derive(Clone, Debug,Eq,PartialEq,Hash,Serialize,Deserialize)]
pub enum Phase {
    /// [`Phase::Unknown`] means [`Foundation::synchronize()`] must be called where the environment
    /// will be probed to determine the present state each of the [`Dependency`] and [`Provider`]
    Unknown,
    /// nothing has been done... not downloaded ... nothing
    None,
    Downloaded,
    Installed,
    Initialized,
    Started,
    Panic
}

impl Default for Phase {
    fn default() -> Self {
        Phase::Unknown
    }
}


/// [`Status`] provides more detailed information than state.  Including ActionRequired which
/// should hopefully tell the user exactly what he needs to do to resolve the issue
#[derive(Clone, Debug,Serialize,Deserialize)]
pub enum Status {
    Unknown,
    None,
    /// meaning that any one of the States: Downloaded, Installed, Initialized are still processing
    Creation,
    ActionRequest(ActionRequest),
    Panic(Panic),
    Ready
}

impl Default for Status{
    fn default() -> Self {
        Status::Unknown
    }
}




#[derive(Clone, Debug,Serialize,Deserialize)]
pub struct Panic {
    pub foundation: FoundationKind,
    pub kind: Kind,
    pub message: String
}

impl Panic {
    pub fn new(foundation: FoundationKind, kind: impl Into<Kind>, provider: Option<CamelCase>, message: String) -> Self {

        let kind = kind.into();
        Self {
            kind,
            foundation,
            message,
        }
    }
}
