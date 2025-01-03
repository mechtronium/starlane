
mod concrete {
    use std::sync::Arc;
    use async_trait::async_trait;
    use sqlx::{Connection, PgPool};
    use sqlx::postgres::PgConnectOptions;
    use starlane_base::foundation::config::ProviderConfig;
    use starlane_base::provider::{ProviderKindDef,ProviderKind};
    use starlane_base::status::{Status,StatusDetail,Handle,StatusEntity,StatusWatcher};
    use starlane_base::provider::{Provider,err::ProviderErr};
    use crate::service::{Pool, PostgresServiceHandle};
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
    }

    impl starlane_hyperspace::provider::config::ProviderConfig for Config {
        fn kind(&self) -> &ProviderKindDef {
            todo!()
        }
    }

    impl ProviderConfig for Config {}


    pub type PostgresDatabaseHandle = Handle<PostgresDatabase>;
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
    }

    #[async_trait]
    impl Provider for PostgresDatabaseProvider {
        type Config = Config;
        type Entity = PostgresDatabase;

        fn kind(&self) -> ProviderKindDef {
            ProviderKindDef::PostgresService
        }

        fn config(&self) -> Arc<Self::Config> {
            self.config.clone()
        }


        async fn ready(&self) -> Result<Self::Entity, ProviderErr> {
            todo!()
        }
    }


    #[async_trait]
    impl StatusEntity for PostgresDatabaseProvider {
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
    }

    #[async_trait]
    impl StatusEntity for PostgresDatabase {
        fn status(&self) -> Status {
            todo!()
        }

        fn status_detail(&self) -> StatusDetail {
            todo!()
        }

        fn status_watcher(&self) -> StatusWatcher {
            todo!()
        }

        async fn probe(&self) -> Status{
            async fn ping(pool: & Pool) -> Result<Status,sqlx::Error> {
                pool.acquire().await?.ping().await.map(|_| Status::Ready)
            }

            match ping(&self.pool).await {
                Ok(_) => Status::Ready,
                Err(_) => Status::Unknown
            }

        }
    }



}