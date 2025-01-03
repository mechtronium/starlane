use std::future::Future;
use std::ops::Deref;
use async_trait::async_trait;
use starlane_space::status::{Entity, Handle, StatusProbe};


/// these reexports must come from [crate::service] since they are mocks when `#[cfg(test)]`
use crate::service::Pool;


pub type PostgresDatabaseHandle = Handle<dyn PostgresDatabase>;


/// tried to make [Handle] expose [Pool] by implementing
/// ```
/// # use std::ops::Deref;
/// # trait Entity {}
/// # struct Handle<E> where E: Entity+Send+Sync { entity: E };
///
/// impl <E,D> Deref for Handle<E> where E: Deref<Target=D>+Entity+Send+Sync {
///     type Target = D;
///
///     fn deref(&self) -> &Self::Target {
///         self.entity.deref()
///     }
/// }
/// ```
#[async_trait]
pub trait PostgresDatabase: Entity+StatusProbe+Deref<Target=Pool>+Send+Sync {

}




mod concrete {
    mod my { pub use super::super::*; }

    use std::ops::Deref;
    use std::sync::Arc;
    use async_trait::async_trait;
    use sqlx::{Connection, PgPool};
    use sqlx::postgres::PgConnectOptions;
    use starlane_base::foundation::config::ProviderConfig;
    use starlane_base::status::{Status, StatusDetail, Handle, StatusProbe, StatusWatcher};
    use starlane_base::provider;
    use provider::{Provider,err::ProviderErr,ProviderKindDef};

    use starlane_base::status::{Entity, EntityReadier, ReadyResult, StatusResult};
    use starlane_space::kind::DatabaseSubKind;
    use super::Pool;
    use crate::service::{PostgresServiceHandle};
    use crate::service::config::{PostgresUtilizationConfig};


    #[derive(Clone, Eq, PartialEq)]
    pub struct Config {
        database: String,
        connection: PostgresUtilizationConfig
    }

    impl Config {
        pub(crate) fn connect_options(&self) -> PgConnectOptions {
            let mut options = self.connection.connect_options();
            options.database(&self.database.as_str())
        }

        pub fn mock() -> Self {
            let database = "registry".to_string();
            let connection = PostgresUtilizationConfig::mock();

            Self {
                database,
               connection,
            }
        }
    }

    impl starlane::config::ProviderConfig for Config {
        fn kind(&self) -> &ProviderKindDef {
            todo!()
        }
    }

    impl ProviderConfig for Config {}


    pub struct PostgresDatabaseProvider {
        config: Arc<Config>,
        status: tokio::sync::watch::Sender<Status>,
    }

    impl PostgresDatabaseProvider {
        pub fn new(config: Arc<Config>) -> Self {
            let (status_reporter, _) = tokio::sync::watch::channel(Default::default());

            Self {
                config,
                status: status_reporter,
            }
        }

        #[cfg(test)]
        pub fn mock() -> Self {
           let config = Arc::new(Config::mock());
           Self::new(config)
        }
    }

    impl Deref for PostgresDatabase {
        type Target = Pool;

        fn deref(&self) -> &Self::Target {
            &self.pool
        }
    }




    #[async_trait]
    impl my::PostgresDatabase for PostgresDatabase { }


    #[async_trait]
    impl EntityReadier for PostgresDatabaseProvider {
        type Entity = PostgresDatabase;

        async fn ready(&self) -> ReadyResult<Self::Entity> {
            todo!()
        }
    }


    #[async_trait]
    impl Provider for PostgresDatabaseProvider {
        type Config = Config;

        fn kind(&self) -> ProviderKindDef {
            ProviderKindDef::PostgresService
        }

        fn config(&self) -> Arc<Self::Config> {
            self.config.clone()
        }

    }

    #[async_trait]
    impl StatusProbe for PostgresDatabaseProvider {
        async fn probe(&self) -> StatusResult {
            todo!()
        }
    }


    pub struct PostgresDatabase {
        config: Config,
        service: PostgresServiceHandle,
        pool: Pool
    }

    impl PostgresDatabase {
        /// create a new Postgres Connection `Pool`
        async fn new(config: Config, service: PostgresServiceHandle) -> Result<Self, sqlx::Error> {
            let pool = PgPool::connect_with(config.connect_options()).await?;

            Ok(Self {
                config,
                service,
                pool
            })
        }

        #[test]
        pub fn mock(service: PostgresServiceHandle) -> Self {

        }

    }


    impl Entity for PostgresDatabase { }

    #[async_trait]
    impl StatusProbe for PostgresDatabase {

        async fn probe(&self) -> StatusResult {
            async fn ping(pool: & Pool) -> Result<Status,sqlx::Error> {
                pool.acquire().await?.ping().await.map(|_| Status::Ready)
            }

            todo!();

            // need to do the hard work of building the actual `StatusDetail`
           /*
            match ping(&self.pool).await {
                Ok(_) => Status::Ready,
                Err(_) => Status::Unknown
            }

            */

        }
    }





}

#[cfg(test)]
pub mod tests {
    use std::ops::Deref;
    use starlane_base::status::{Entity, Handle, StatusProbe, StatusResult};
    use super::concrete::{PostgresDatabase, PostgresDatabaseProvider};

    #[tokio::test]
    pub async fn test_handle_deref() {
        let service_provider = PostgresDatabaseProvider::mock();
        let database = PostgresDatabase::mock();
        let handle = Handle::mock(database);

        let deref = handle.deref();

    }
}