use std::fmt::Display;
use starlane_space::parse::{Domain, VarCase};
use std::sync::Arc;
use async_trait::async_trait;
use starlane_base_common::config::ProviderConfig;
use starlane_base_common::provider::{Manager, Provider, ProviderKindDef};
use starlane_base_common::provider::err::ProviderErr;
use starlane_space::err::ParseErrs;
use std::str::FromStr;
use starlane_base_common::Foundation;
use starlane_base_common::platform::prelude::Platform;
use starlane_base_common::status::{Handle,Status,StatusDetail,StatusEntity,StatusWatcher};


/// The [Platform]  implementation of [PostgresService].
///
/// [PostgresService] provides access to a Postgres Cluster Instance.
///
/// This mod implements the platform [PostgresService] which is a [Provider] that readies a
/// [PostgresServiceHandle].  Like every platform provider this [PostgresService] implementation
/// cannot install 3rd party extensions, a platform [Provider] CAN maintain a connection pool
/// to a postgres cluster that already exists or if the [Foundation] has a [Provider] definition of
/// with a matching [ProviderKindDef]... the [Foundation] [Provider] can be a dependency of the
/// [Platform]

pub type PostgresServiceHandle = Handle<PostgresServiceStub>;


pub struct PostgresService {
    config: Arc<Config>,
    status_reporter: tokio::sync::watch::Sender<Status>,
}

impl PostgresService {
    pub fn new(config: Arc<Config>) -> PostgresService {
        let (status_reporter, _ ) = tokio::sync::watch::channel(Default::default());


        Self {
            config,
            status_reporter,
        }
    }
}

#[async_trait]
impl Provider for PostgresService {
    type Config = Config;
    type Item = PostgresServiceHandle;

    fn kind(&self) -> ProviderKindDef {
        ProviderKindDef::PostgresService
    }

    fn config(&self) -> Arc<Self::Config> {
        self.config.clone()
    }

    async fn probe(&self) -> Result<(), ProviderErr> {
        todo!()
    }

    async fn ready(&self) -> Result<Self::Item, ProviderErr> {
        todo!()
    }
}


#[async_trait]
impl StatusEntity for PostgresService {
    fn status(&self) -> Status {
        todo!()
    }

    fn status_detail(&self) -> StatusDetail {
        todo!()
    }

    fn status_watcher(&self) -> StatusWatcher {
        todo!()
    }

    async fn probe(&self) -> StatusWatcher {
        todo!()
    }


}

/// the [StatusEntity] implementation which tracks with a Postgres Connection Pool.
/// With any [StatusEntity] the goal is to get to a [Status::Ready] state.  [PostgresServiceStub]
/// should abstract the specific [Manager] details.  A [PostgresServiceStub] may be a
/// [Manager::Foundation] in which the [PostgresServiceStub] would be responsible for
/// downloading, installing, initializing and starting Postgres before it creates the pool or if
/// [Manager::External] then Starlane's [Platform] is only responsible for maintaining
/// a connection pool to the given Postgres Cluster
pub struct PostgresServiceStub {
    key: DbKey,
    connection_info: Config,
}

#[async_trait]
impl StatusEntity for PostgresServiceStub {
    fn status(&self) -> Status {
        todo!()
    }

    fn status_detail(&self) -> StatusDetail {
        todo!()
    }

    fn status_watcher(&self) -> StatusWatcher {
        todo!()
    }

    async fn probe(&self) -> StatusWatcher {
        todo!()
    }
}

impl PostgresServiceStub {
    pub fn key(&self) -> &DbKey {
        &self.key
    }
}

/// maybe add proper postgres type constraints on the following stuff:
pub type Username = VarCase;
pub type Password = String;
pub type DbName = VarCase;
/// default to 'public'
pub type SchemaName = VarCase;
pub type Hostname = Domain;

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct DbKey {
    pub host: Hostname,
    pub user: Username,
    pub database: DbName,
    /// default to public if [None]
    pub schema: Option<SchemaName>,
}

impl Display for DbKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            format!("{}:{}@{}", self.user, self.database, self.host)
        )
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum PostErr {
    #[error("{0}")]
    ParseErrs(#[from] ParseErrs),
}

#[derive(Clone, Eq, PartialEq)]
pub struct Config {
    pool: PostgresConnectionConfig
}

impl Config {

}

impl ProviderConfig for Config {
    fn kind(&self) -> &ProviderKindDef {
        &ProviderKindDef::PostgresService
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct PostgresConnectionConfig {
    pub host: Hostname,
    pub port: u16,
    pub username: Username,
    pub password: String,
}

impl PostgresConnectionConfig{
    pub fn new<User, Pass>(
        host: Hostname,
        port: u16,
        username: User,
        password: Pass,
    ) -> Result<Self, PostErr>
    where
        User: AsRef<str>,
        Pass: ToString,
    {
        let username = Username::from_str(username.as_ref())?;
        let password = password.to_string();
        Ok(Self {
            host,
            username,
            password,
            port,
        })
    }


}