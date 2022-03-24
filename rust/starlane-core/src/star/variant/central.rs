use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use mesh_portal::version::latest::entity::request::create::Strategy;

use tokio::sync::{mpsc, oneshot};
use crate::command::cli::{CliServer, inlet, outlet};


use crate::error::Error;
use crate::resource::{Kind, ResourceRecord,  ResourceLocation};
use crate::star::{StarKey, StarSkel};
use crate::star::variant::{FrameVerdict, VariantCall};
use crate::starlane::api::StarlaneApi;
use crate::user::HyperUser;
use crate::util::{AsyncProcessor, AsyncRunner};

static BOOT_BUNDLE_ZIP : &'static [u8] = include_bytes!("../../../boot/bundle.zip");

pub struct CentralVariant {
    skel: StarSkel,
    initialized: bool
}

impl CentralVariant {
    pub fn start(skel: StarSkel, rx: mpsc::Receiver<VariantCall>) {
        AsyncRunner::new(
            Box::new(Self { skel: skel.clone(),
            initialized: false}),
            skel.variant_api.tx.clone(),
            rx,
        );
    }
}

#[async_trait]
impl AsyncProcessor<VariantCall> for CentralVariant {
    async fn process(&mut self, call: VariantCall) {
        match call {
            VariantCall::Init(tx) => {
                self.init_variant(tx);
            }
            VariantCall::Frame { frame, session:_, tx } => {
                tx.send(FrameVerdict::Handle(frame));
            }
        }
    }
}


impl CentralVariant {
    fn init_variant(&mut self, tx: oneshot::Sender<Result<(), Error>>) {
        if self.initialized == true {
            tx.send(Ok(()));
            return;
        } else {
            self.initialized = true;
        }

        let skel = self.skel.clone();

        tokio::spawn(async move {
            skel.sys_api.create( HyperUser::template(), HyperUser::messenger()  ).await;

            let starlane_api = StarlaneApi::new(skel.surface_api.clone(), skel.info.address.clone() );
            let result = Self::ensure(starlane_api).await;
            if let Result::Err(error) = result.as_ref() {
                error!("Central Init Error: {}", error.to_string());
            }
            tx.send(result);
        });
    }
}

impl CentralVariant {
    async fn ensure(starlane_api: StarlaneApi) -> Result<(), Error> {
        let mut creation = starlane_api.create_space("hyperspace").await?;
        creation.set_strategy(Strategy::Ensure);
        creation.submit().await?;

        let mut creation = starlane_api.create_space("localhost" ).await?;
        creation.set_strategy(Strategy::Ensure);
        creation.submit().await?;

        let (tx,mut rx) = CliServer::new_internal( starlane_api ).await?;

        tx.send(inlet::Frame::CommandLine("? create hyperspace:repo<Base<Repo>>".to_string()) ).await?;
        tx.send(inlet::Frame::EndRequires ).await?;
        while let Some(frame) = rx.recv().await {
            if let outlet::Frame::EndOfCommand(_) = frame {
                break;
            }
        }
        tx.send(inlet::Frame::CommandLine("? create hyperspace:repo:boot<ArtifactBundleSeries>".to_string()) ).await?;
        tx.send(inlet::Frame::EndRequires ).await?;
        while let Some(frame) = rx.recv().await {
            if let outlet::Frame::EndOfCommand(_) = frame {
                break;
            }
        }

        tx.send(inlet::Frame::CommandLine("? publish ^[ bundle.zip ]-> hyperspace:repo:boot:1.0.0".to_string()) ).await?;
        let content = Arc::new( BOOT_BUNDLE_ZIP.to_vec() );
        tx.send(inlet::Frame::TransferFile { name: "bundle.zip".to_string(), content }).await?;
        tx.send(inlet::Frame::EndRequires ).await?;

        while let Some(frame) = rx.recv().await {
            if let outlet::Frame::EndOfCommand(_) = frame {
                break;
            }
        }

        tx.send(inlet::Frame::CommandLine("? create hyperspace:users<UserBase<Keycloak>>".to_string()) ).await?;
        tx.send(inlet::Frame::EndRequires ).await?;
        while let Some(frame) = rx.recv().await {
            if let outlet::Frame::EndOfCommand(_) = frame {
                break;
            }
        }

        tx.send(inlet::Frame::CommandLine("? create hyperspace:users:hyperuser<User>".to_string()) ).await?;
        tx.send(inlet::Frame::EndRequires ).await?;
        Ok(())
    }
}
