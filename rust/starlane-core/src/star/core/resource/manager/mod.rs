use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};

use artifact::ArtifactBundleManager;
use k8s::KubeManager;

use crate::mesh::serde::payload::Payload;
use crate::error::Error;
use crate::mesh::{Request, Response};
use crate::mesh::serde::entity::request::Rc;
use crate::mesh::serde::id::{Address, ResourceType};
use crate::message::delivery::Delivery;
use crate::resource;
use crate::resource::ResourceAssign;
use crate::star::StarSkel;
use crate::util::{AsyncProcessor, Call, AsyncRunner};
use crate::star::core::resource::manager::stateless::StatelessManager;
use crate::star::core::resource::manager::app::AppManager;
use crate::star::core::resource::manager::mechtron::MechtronManager;
use crate::star::core::resource::manager::file::{FileSystemManager, FileManager};
use std::collections::HashMap;

mod stateless;
pub mod artifact;
mod default;
pub mod file_store;
pub mod k8s;
pub mod mechtron;
pub mod app;
pub mod file;

#[derive(Clone)]
pub struct ResourceManagerApi {
    pub tx: mpsc::Sender<ResourceManagerCall>,
}

impl ResourceManagerApi {
    pub fn new(tx: mpsc::Sender<ResourceManagerCall>) -> Self {
        Self { tx }
    }

    pub async fn assign( &self, assign: ResourceAssign) -> Result<(),Error> {
        let (tx,rx) = oneshot::channel();
        self.tx.send(ResourceManagerCall::Assign{assign, tx });
        rx.await?
    }

    pub async fn has( &self, address: Address) -> Result<bool,Error> {
        let (tx,rx) = oneshot::channel();
        self.tx.send(ResourceManagerCall::Has{address, tx });
        Ok(rx.await?)
    }

    pub async fn request( &self, request: Delivery<Request>) {
        let (tx,rx) = oneshot::channel();
        self.tx.send(ResourceManagerCall::Request{request, tx });
    }

    pub async fn get( &self, address: Address ) -> Result<Payload,Error> {
        let (tx,rx) = oneshot::channel();
        self.tx.send(ResourceManagerCall::Get{address, tx });
        rx.await?
    }
}

pub enum ResourceManagerCall {
    Assign{ assign:ResourceAssign, tx: oneshot::Sender<Result<(),Error>> },
    Has { address: Address, tx: oneshot::Sender<bool> },
    Request { request: Delivery<Request>, tx: oneshot::Sender<Result<Option<Response>,Error>>},
    Get{ address: Address, tx: oneshot::Sender<Result<Payload,Error>>}
}


impl Call for ResourceManagerCall {}



pub struct ResourceManagerComponent {
    pub skel: StarSkel,
    managers: HashMap<ResourceType,Arc<dyn ResourceManager>>,
    resources: HashMap<Address,ResourceType>
}

impl ResourceManagerComponent {
    pub fn new( skel: StarSkel, rx: mpsc::Receiver<ResourceManagerCall> ) {
        AsyncRunner::new(
        Box::new(Self {
            skel,
            managers: HashMap::new(),
            resources: HashMap::new()
        }),skel.resource_manager_api.tx.clone(), rx);
    }
}

#[async_trait]
impl AsyncProcessor<ResourceManagerCall> for ResourceManagerComponent{
    async fn process(&mut self, call: ResourceManagerCall) {
        match call {
            ResourceManagerCall::Assign { assign, tx } => {

            }
            ResourceManagerCall::Has { address, tx } => {}
            ResourceManagerCall::Request { request, tx } => {}
            ResourceManagerCall::Get { address, tx } => {}
        }
    }
}

impl ResourceManagerComponent{

    async fn assign( &mut self, assign: ResourceAssign, tx: mpsc::Sender<Result<(),Error>> ) {

       async fn process( manager_component: &mut ResourceManagerComponent, assign: ResourceAssign) -> Result<(),Error> {
           let manager = manager_component.manager(&assign.kind.resource_type() ).await?;
           manager_component.resources.insert( assign.stub.address.clone(), assign.stub.kind.resource_type() );
           manager.assign(assign).await
       }

       tx.send( process(self,assign).await );
    }

    async fn request( &mut self, request: Delivery<Request>) {

        async fn process( manager_component: &mut ResourceManagerComponent, request: Delivery<Request>) -> Result<(),Error> {
            let resource_type = manager_component.resources.get(&request.to ).ok_or("could not find resource".into() )?;
            let manager = manager_component.manager(resource_type ).await?;
            manager.handle_request(request ).await?;
            Ok(())
        }

        match process( self, request.clone() ).await {
            Ok(_) => {
                // no need to do anything
            }
            Err(err) => {
                request.fail(err.into());
            }
        }
    }

    async fn has( &mut self, address: Address, tx: mpsc::Sender<bool> ) {
        tx.send( self.resources.contains_key(&address)  );
    }

    async fn manager(&mut self, rt: &ResourceType) -> Result<Arc<dyn ResourceManager>,Error> {

        if self.managers.contains_key(rt) {
            Ok(self.managers.get(rt).cloned().ok_or("expected reference to shell".into()));
        }

        let manager: Arc<dyn ResourceManager> = match rt {
            ResourceType::Space => Arc::new(StatelessManager::new(self.skel.clone(), ResourceType::Space ).await),
            ResourceType::ArtifactBundleSeries => Arc::new(StatelessManager::new(self.skel.clone(), ResourceType::ArtifactBundleSeries).await),
            ResourceType::ArtifactBundle=> Arc::new(ArtifactBundleManager::new(self.skel.clone()).await),
            ResourceType::App=> Arc::new(AppManager::new(self.skel.clone()).await),
            ResourceType::Mechtron => Arc::new(MechtronManager::new(self.skel.clone()).await),
            ResourceType::Database => Arc::new(KubeManager::new(self.skel.clone(), ResourceType::Database ).await.expect("KubeHost must be created without error")),
            ResourceType::FileSystem => Arc::new(FileSystemManager::new(self.skel.clone() ).await),
            ResourceType::File => Arc::new(FileManager::new(self.skel.clone()).await),

            t => return Err(format!("no HOST implementation for type {}", t.to_string()).into()),
        };

        self.managers.insert(rt.clone(), manager.clone() );
        Ok(manager)
    }
}

#[async_trait]
pub trait ResourceManager: Send + Sync {

    fn resource_type(&self) -> resource::ResourceType;


    async fn assign(
        &self,
        assign: ResourceAssign,
    ) -> Result<(),Error>;


    fn handle_request(&self, request: Delivery<Request> ) {
        // delivery.fail(fail::Undeliverable)
    }

    async fn has(&self, address: Address) -> bool;

    fn shutdown(&self) {}

}
