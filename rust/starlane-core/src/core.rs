use std::{thread};












use tokio::runtime::{Builder};
use tokio::sync::{mpsc, oneshot};



use starlane_resources::ResourceIdentifier;

use crate::core::artifact::ArtifactHost;
use crate::core::default::DefaultHost;
use crate::core::file_store::FileStoreHost;
use crate::core::kube::KubeCore;
use crate::error::Error;

use crate::frame::MessagePayload;

use crate::message::Fail;
use crate::resource::{
    AssignResourceStateSrc, HostedResource, HostedResourceStore, LocalHostedResource,
    RemoteDataSrc, Resource, ResourceAssign, ResourceSliceAssign,
};

use crate::star::{
    ActorCreate, LocalResourceLocation, Request, StarCommand, StarKey, StarKind, StarSkel,
};
use crate::data::{NetworkBinSrc, DataSetSrc, LocalBinSrc};
use crate::data::DataSet;


pub mod artifact;
pub mod default;
pub mod file_store;
pub mod server;
mod kube;



pub struct StarCoreAction {
    pub command: StarCoreCommand,
    pub tx: oneshot::Sender<Result<StarCoreResult, Fail>>,
}

impl ToString for StarCoreAction{
    fn to_string(&self) -> String {
        self.command.to_string()
    }
}

impl StarCoreAction {
    pub fn new(
        command: StarCoreCommand,
    ) -> (Self, oneshot::Receiver<Result<StarCoreResult, Fail>>) {
        let (tx, rx) = oneshot::channel();
        (
            StarCoreAction {
                command: command,
                tx: tx,
            },
            rx,
        )
    }
}

#[derive(strum_macros::Display)]
pub enum StarCoreCommand {
    Get(ResourceIdentifier),
    State(ResourceIdentifier),
    Assign(ResourceAssign<AssignResourceStateSrc<DataSetSrc<LocalBinSrc>>>),
    Shutdown
}

pub enum StarCoreResult {
    Ok,
    Resource(Option<Resource>),
    LocalLocation(LocalResourceLocation),
    MessageReply(MessagePayload),
    State(DataSetSrc<LocalBinSrc>),
}

impl ToString for StarCoreResult {
    fn to_string(&self) -> String {
        match self {
            StarCoreResult::Ok => "Ok".to_string(),
            StarCoreResult::LocalLocation(_) => "LocalLocation".to_string(),
            StarCoreResult::MessageReply(_) => "MessageReply".to_string(),
            StarCoreResult::Resource(_) => "Resource".to_string(),
            StarCoreResult::State(_) => "State".to_string(),
        }
    }
}

pub enum CoreRunnerCommand {
    Core {
        skel: StarSkel,
        rx: mpsc::Receiver<StarCoreAction>,
    },
    Shutdown,
}

pub struct CoreRunner {
    tx: mpsc::Sender<CoreRunnerCommand>,
}

impl CoreRunner {
    pub fn new() -> Result<Self, Error> {

        let factory = StarCoreFactory::new();
        let (tx, mut rx) = mpsc::channel(1);
        thread::spawn(move || {

            let runtime = Builder::new_multi_thread()
                .worker_threads(4)
                .thread_name("star-core-runner")
                .thread_stack_size(3 * 1024 * 1024)
                .enable_all()
                .build()
                .unwrap();


            runtime.block_on(async move {
                while let Option::Some(CoreRunnerCommand::Core { skel, rx }) = rx.recv().await {
                    let core = match factory.create(skel, rx).await {
                        Ok(core) => core,
                        Err(err) => {
                            error!("FATAL: {}", err);
                            panic!("FATAL: {}", err);
//                            std::process::exit(1);
                        }
                    };
                    tokio::spawn(async move {
                        core.run().await;
                    });
                }
            });
        });


        Ok(CoreRunner { tx: tx })
    }

    pub async fn send(&self, command: CoreRunnerCommand) {
        self.tx.send(command).await;
    }
}

#[async_trait]
pub trait StarCoreExt: Sync + Send {}

#[async_trait]
pub trait StarCore: Sync + Send {
    async fn run(&mut self);
}

pub struct StarCoreFactory {}

impl StarCoreFactory {
    pub fn new() -> Self {
        StarCoreFactory {}
    }

    pub async fn create(
        &self,
        skel: StarSkel,
        core_rx: mpsc::Receiver<StarCoreAction>,
    ) -> Result<StarCore2, Error> {
        let file_access = skel.data_access.clone();

        let host: Box<dyn Host> = match skel.info.kind {
            StarKind::FileStore => {
                Box::new(FileStoreHost::new(skel.clone(), file_access).await?)
            },
            StarKind::ArtifactStore => {
                Box::new(ArtifactHost::new(skel.clone(), file_access).await?)
            }
            StarKind::Kube => {
                Box::new(KubeCore::new(skel.clone()).await?)
            }
            _ => Box::new(DefaultHost::new(skel.clone()).await),
        };
        Ok(StarCore2::new(skel, core_rx, host).await)
    }
}

pub struct InertHost {}

impl InertHost {
    pub fn new() -> Self {
        InertHost {}
    }
}

#[async_trait]
impl Host for InertHost {
    async fn assign(
        &mut self,
        _assign: ResourceAssign<AssignResourceStateSrc<DataSetSrc<LocalBinSrc>>>,
    ) -> Result<Resource, Fail> {
        Err(Fail::Error(
            "This is an InertHost which cannot actually host anything".into(),
        ))
    }

    async fn get(&self, _identifier: ResourceIdentifier) -> Result<Option<Resource>, Fail> {
        Err(Fail::Error(
            "This is an InertHost which cannot actually host anything".into(),
        ))
    }

    async fn state(&self, identifier: ResourceIdentifier) -> Result<DataSetSrc<LocalBinSrc>, Fail> {
        Err(Fail::Error(
            "This is an InertHost which cannot actually host anything".into(),
        ))
    }


    async fn delete(&self, _identifier: ResourceIdentifier) -> Result<(), Fail> {
        Err(Fail::Error(
            "This is an InertHost which cannot actually host anything".into(),
        ))
    }
}

/*
pub struct InertStarCore {
}

#[async_trait]
impl StarCore for InertStarCore
{
    async fn run(&mut self){
    }
}

impl InertStarCore {
    pub fn new()->Self {
        InertStarCore {}
    }
}

 */

/*
pub trait StarCoreExtFactory: Send+Sync
{
    fn create( &self, skell: &StarSkel ) -> StarCoreExtKind;
}

 */

#[async_trait]
pub trait Host: Send + Sync {
    async fn assign(
        &mut self,
        assign: ResourceAssign<AssignResourceStateSrc<DataSetSrc<LocalBinSrc>>>,
    ) -> Result<Resource, Fail>;
    async fn get(&self, identifier: ResourceIdentifier) -> Result<Option<Resource>, Fail>;
    async fn state(&self, identifier: ResourceIdentifier) -> Result<DataSetSrc<LocalBinSrc>, Fail>;
    async fn delete(&self, identifier: ResourceIdentifier) -> Result<(), Fail>;
    fn shutdown(&self) {}
}

pub struct StarCore2 {
    skel: StarSkel,
    rx: mpsc::Receiver<StarCoreAction>,
    host: Box<dyn Host>,
}

impl StarCore2 {
    pub async fn new(
        skel: StarSkel,
        rx: mpsc::Receiver<StarCoreAction>,
        host: Box<dyn Host>,
    ) -> Self {
        StarCore2 {
            skel: skel,
            rx: rx,
            host: host,
        }
    }


    pub async fn run(mut self) {
        while let Option::Some(action) = self.rx.recv().await {
            if let StarCoreCommand::Shutdown = action.command  {
                self.process(action.command).await;
                break;
            }
            let result = self.process(action.command).await;
            action.tx.send(result);
        }
    }


    /*
    pub async fn run(mut self) {
        info!("CORE running");
        loop {
            let tick = tokio::time::timeout(Duration::from_secs(5), self.rx.recv()).await;
            if let Result::Ok(Option::Some(action)) = tick {
                info!("processing {}", action.to_string());
                let result = self.process(action.command).await;
                action.tx.send(result);
                info!("looping...");
            } else {
                info!("{} tick!", self.skel.info.to_string());
            }

        }
        error!("CoreRunner terminated")
    }

     */



    async fn process(&mut self, command: StarCoreCommand) -> Result<StarCoreResult, Fail> {
        match command {
            StarCoreCommand::Assign(assign) => Ok(StarCoreResult::Resource(Option::Some(
                self.host.assign(assign).await?,
            ))),
            StarCoreCommand::Get(identifier) => {
                let resource = self.host.get(identifier).await?;
                Ok(StarCoreResult::Resource(resource))
            }
            StarCoreCommand::State(identifier) => {
                let state_src = self.host.state(identifier).await?;
                Ok(StarCoreResult::State(state_src))
            }
            StarCoreCommand::Shutdown => {
                self.host.shutdown();
                Ok(StarCoreResult::Ok)
            }
        }
    }
}
