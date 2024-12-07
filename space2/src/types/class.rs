use crate::schema::case::CamelCase;
use crate::types::private;
use crate::types::Cat;
use alloc::string::ToString;
use core::str::FromStr;
use strum_macros::EnumDiscriminants;

#[derive(Clone, Debug, Eq, PartialEq, Hash, EnumDiscriminants, strum_macros::Display)]
#[strum_discriminants(vis(pub))]
#[strum_discriminants(name(Variant))]
#[strum_discriminants(derive(
    Hash,
    strum_macros::EnumString,
    strum_macros::ToString,
    strum_macros::IntoStaticStr
))]
pub enum Class {
    Root,
    Platform,
    Foundation,
    /// Dependencies are external bits that can be downloaded and added to a Starlane instance.
    /// Adding new capabilities to Starlane via external software is the main intended use case
    /// for a Dependency (both Foundation binaries and WebAssembly alike)
    ///
    /// A Dependency can be `downloaded`, `installed`, `initialized` and `started` (what those
    /// phases actually mean to the Dependency itself is custom behavior)  The job of the Dependency
    /// create the prerequisite conditions for it child [Class::Provider]s
    Dependency,
    /// Provider `provides` something to Starlane.  Providers enable Starlane to extend itself by
    /// Providing new functionality that the core Starlane binary did not ship with.
    ///
    /// In particular Providers are meant to install WebAssembly Components, Drivers for new
    /// Classes, 3rd party software implementations... etc;
    ///
    Provider,
    /// meaning the [Host] of an execution environment, VM, Wasm host
    /// [Host] is a class and a layer in the message traversal
    Host,
    /// The [Guest] which executes inside a [Host].  A single Guest instance may provide the execution
    /// for any number of other Classes that it provides
    Guest,
    Plugin,
    Service,
    Global,
    Registry,
    Star,
    Driver,
    Portal,
    Control,
    App,
    Wasm,
    Repository,
    Artifact,
    Base,
    User,
    Role,
    Group,
    FileStore,
    Directory,
    File,
    #[strum(to_string = "{0}")]
    _Ext(CamelCase),
}

impl FromStr for Class {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn ext(s: &str) -> Result<Class, eyre::Error> {
            Ok(Class::_Ext(CamelCase::from_str(s)?.into()))
        }

        match Variant::from_str(s) {
            /// this Ok match is actually an Error!
            Ok(Variant::_Ext) => ext(s),
            Ok(variant) => ext(variant.into()),
            Err(_) => ext(s),
        }
    }
}

impl private::Variant for Class {}

impl From<CamelCase> for Class {
    fn from(src: CamelCase) -> Self {
        /// it should not be possible for this to fail
        Self::from_str(src.as_str()).unwrap()
    }
}

impl private::Typical for Class {
    fn category(&self) -> Cat {
        Cat::Class
    }
}

impl Into<CamelCase> for Class {
    fn into(self) -> CamelCase {
        CamelCase::from_str(self.to_string().as_str()).unwrap()
    }
}

impl Into<Cat> for Class {
    fn into(self) -> Cat {
        Cat::Class
    }
}

#[cfg(feature = "parse")]
mod parse {
    use crate::err::ErrStrata;
    use crate::schema::case::CamelCase;
    use crate::types::class::Class;
    use core::str::FromStr;

    impl FromStr for Class {
        type Err = ErrStrata;

        fn from_str(src: &str) -> Result<Self, Self::Err> {
            Ok(Self(CamelCase::from_str(src)?))
        }
    }
}
