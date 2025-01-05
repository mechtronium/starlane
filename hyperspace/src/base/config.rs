use crate::base::provider::Provider;
use crate::base::provider::ProviderKind;
use std::collections::HashSet;
use std::hash::Hash;

use crate::base::err::BaseErr;
use crate::base::{kinds, provider, BaseSub};
use crate::registry;

/// a container for all sub-strata layers
pub trait BaseConfig: Send + Sync {
    type Err: Into<BaseErr>;
    type PlatformConfig: PlatformConfig + ?Sized;
    type FoundationConfig: FoundationConfig + ?Sized;

    fn foundation(&self) -> Self::FoundationConfig;
    fn platform(&self) -> Self::PlatformConfig;
}

//
pub trait BaseSubConfig: Send + Sync {
    type Kind: kinds::Kind + ?Sized;

    fn kind(&self) -> &Self::Kind;
}

pub trait FoundationConfig: BaseSubConfig {
    type Provider: Provider + ?Sized;

    fn required(&self) -> HashSet<ProviderKind>;

    fn provider_kinds(&self) -> &HashSet<ProviderKind>;

    fn provider<K>(&self, kind: &<<Self::Provider as BaseSub>::Config as BaseSubConfig>::Kind) -> Option<&Self::Provider>;
}

pub trait ProviderConfig: provider::config::ProviderConfig { }

pub trait PlatformConfig: crate::base::PlatformConfig { }

pub trait RegistryConfig: registry::RegistryConfig { }
