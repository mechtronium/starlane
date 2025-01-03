//! The [Platform]  implementation of [Provider].
//!
//! [Provider] provides access to a Postgres Cluster Instance.
//!
//! This mod implements the platform [Provider] which is a [provider::Provider] that readies a
//! [PostgresServiceHandle].  Like every platform provider this [Provider] implementation
//! cannot install 3rd party extensions, a platform [provider::Provider] CAN maintain a connection pool
//! to a postgres cluster that already exists or if the [Foundation] has a [provider::Provider] definition of
//! with a matching [ProviderKindDef]... the [Foundation] [provider::Provider] can be a dependency of the
//! [Platform]

pub type Pool = sqlx::Pool<sqlx::Postgres>;
pub type Con = sqlx::pool::PoolConnection<sqlx::Postgres>;

/// maybe add proper postgres type constraints on the following stuff:
pub type Username = VarCase;
pub type Password = String;
pub type DbName = VarCase;
/// default to 'public'
pub type SchemaName = VarCase;
pub type Hostname = Domain;

use std::fmt::Display;
use std::sync::Arc;
use async_trait::async_trait;
use sqlx::postgres::PgConnectOptions;
use starlane_base::provider;
use starlane_space::parse::{Domain, VarCase};
use starlane_space::status::{Handle, StatusEntity};
use starlane_base::Foundation;
use starlane_base::platform::prelude::Platform;
use starlane_base::kind::ProviderKindDef;

/// final [provider::config::ProviderConfig] trait definitions for [concrete::PostgresProviderConfig]
#[async_trait]
pub trait ProviderConfig:  provider::config::ProviderConfig  {
    fn utilization_config(&self) ->  & config::PostgresUtilizationConfig;

    /// reexport [config::PostgresUtilizationConfig::connect_options]
    fn connect_options(&self) -> PgConnectOptions {
        self.utilization_config().connect_options()
    }
}

/// final [provider::Provider] trait definitions for [concrete::PostgresServiceProvider]
#[async_trait]
pub trait Provider:  provider::Provider<Entity=PostgresServiceHandle>  {
    type Config: ProviderConfig + ?Sized;
}


/// trait implementation [Provider::Entity]
#[async_trait]
pub trait PostgresService : StatusEntity {}


pub type PostgresServiceHandle = Handle<Arc<dyn PostgresService>>;


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

pub mod config {
    mod my { pub use super::super::*; }
    use std::str::FromStr;
    use sqlx::postgres::PgConnectOptions;
    use crate::err::PostErr;

    #[derive(Clone, Eq, PartialEq)]
    pub struct PostgresUtilizationConfig {
        pub host: my::Hostname,
        pub port: u16,
        pub username: my::Username,
        pub password: String,
    }

    impl PostgresUtilizationConfig {
        pub fn new<User, Pass>(
            host: my::Hostname,
            port: u16,
            username: User,
            password: Pass,
        ) -> Result<Self, PostErr>
        where
            User: AsRef<str>,
            Pass: ToString,
        {
            let username = my::Username::from_str(username.as_ref())?;
            let password = password.to_string();
            Ok(Self {
                host,
                username,
                password,
                port,
            })
        }

        pub(crate) fn connect_options(&self) -> PgConnectOptions {
            PgConnectOptions::new()
                .host(self.host.as_str())
                .port(self.port.clone())
                .username(self.username.as_str())
                .password(self.password.as_str())
        }

    }

}



pub mod partial {
    pub mod mount {
    }
}




mod concrete {
    use std::fmt::Display;
    use std::ops::Deref;
    use std::sync::Arc;
    use async_trait::async_trait;
    use starlane_base::config::ProviderConfig;
    use starlane_base::provider::{Manager, Provider, ProviderKindDef};
    use starlane_base::provider::err::ProviderErr;
    use std::str::FromStr;
    use sqlx;
    use sqlx::{ConnectOptions, Connection};
    use tokio::sync::Mutex;
    use starlane_base::Foundation;
    use starlane_base::platform::prelude::Platform;
    use starlane_space::status;
    use status::{EntityResult,Handle, Status, StatusDetail, StatusEntity, StatusWatcher};

    use crate::service::config::PostgresUtilizationConfig;

    pub mod my { pub use super::super::*;}


    pub struct PostgresServiceProvider {
        config: Arc<PostgresProviderConfig>,
        status: tokio::sync::watch::Sender<Status>,
    }

    impl PostgresServiceProvider {
        pub fn new(config: Arc<PostgresProviderConfig>) -> PostgresServiceProvider {
            let (status_reporter, _) = tokio::sync::watch::channel(Default::default());

            Self {
                config,
                status: status_reporter,
            }
        }
    }

    #[async_trait]
    impl Provider for PostgresServiceProvider {
        type Config = PostgresProviderConfig;
        type Entity = my::PostgresServiceHandle;

        fn kind(&self) -> ProviderKindDef {
            ProviderKindDef::PostgresService
        }

        fn config(&self) -> Arc<Self::Config> {
            self.config.clone()
        }

        async fn ready(&self) -> EntityResult<Self::Entity> {
            todo!()
        }
    }


    #[async_trait]
    impl StatusEntity for PostgresServiceProvider {
        fn status(&self) -> Status {
            todo!()
        }

        fn status_detail(&self) -> status::Result {
            todo!()
        }

        fn status_watcher(&self) -> StatusWatcher {
            todo!()
        }

        async fn probe(&self) -> status::Result {
            todo!()
        }
    }

    /// the [StatusEntity] implementation which tracks with a Postgres Connection Pool.
    /// With any [StatusEntity] the goal is to get to a [Status::Ready] state.  [PostgresService]
    /// should abstract the specific [Manager] details.  A [PostgresService] may be a
    /// [Manager::Foundation] in which the [PostgresService] would be responsible for
    /// downloading, installing, initializing and starting Postgres before it creates the pool or if
    /// [Manager::External] then Starlane's [Platform] is only responsible for maintaining
    /// a connection pool to the given Postgres Cluster
    pub struct PostgresService {
        config: PostgresProviderConfig,
        connection: Mutex<sqlx::PgConnection>
    }

    #[async_trait]
    impl my::PostgresService for PostgresService { }


    impl PostgresService {
        async fn new(config: PostgresProviderConfig) -> Result<Self, sqlx::Error> {
            let connection = Mutex::new(config.connect_options().connect().await?);
            Ok(Self {
                config,
                connection
            })
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

        async fn probe(&self) -> Status {
            /// need to normalize the [PostgresService::probe]
            self.connection.lock().await.ping().await.unwrap();
            todo!()
        }
    }




    #[derive(Clone, Eq, PartialEq)]
    pub struct PostgresProviderConfig {
        connection_info: my::config::PostgresUtilizationConfig
    }


    #[async_trait]
    impl my::ProviderConfig for PostgresProviderConfig {
        fn utilization_config(&self) -> & PostgresUtilizationConfig {
            & self.connection_info
        }
    }

    impl Deref for PostgresProviderConfig {
        type Target = my::config::PostgresUtilizationConfig;

        fn deref(&self) -> &Self::Target {
            &self.connection_info
        }
    }

    impl PostgresProviderConfig {}

    #[async_trait]
    impl ProviderConfig for PostgresProviderConfig {
        fn kind(&self) -> &ProviderKindDef {
            &ProviderKindDef::PostgresService
        }
    }


}