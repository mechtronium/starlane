use std::sync::Arc;
use std::sync::atomic::AtomicI32;

use futures::future::{err, join_all, ok, select_all};
use futures::FutureExt;
use futures::prelude::*;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{mpsc, Mutex, broadcast};

use crate::constellation::Constellation;
use crate::error::Error;
use crate::id::Id;
use crate::lane::{STARLANE_PROTOCOL_VERSION, Tunnel};
use crate::message::{ProtoGram, LaneGram};
use crate::star::{Star, StarKernel, StarKey, StarShell, StarKind, StarCommand, StarController};
use std::cell::RefCell;

pub struct ProtoStar
{
  proto_lanes: Vec<ProtoTunnel>,
  lane_seq: AtomicI32,
  kind: StarKind,
  key: StarKey,
  command_rx: Receiver<StarCommand>
}

impl ProtoStar
{
    pub fn new(key: StarKey, kind: StarKind) ->(Self, StarController)
    {
        let (command_tx, command_rx) = mpsc::channel(32);
        (ProtoStar{
            lane_seq: AtomicI32::new(0),
            proto_lanes: vec![],
            kind,
            key,
            command_rx: command_rx
        },StarController{
            command_tx: command_tx
        })
    }

    pub fn add_lane( &mut self, proto_lane: ProtoTunnel)
    {
        self.proto_lanes.push(proto_lane);
    }

    pub async fn evolve(&mut self)->Result<Arc<Star>,Error>
    {
        let mut lanes = vec![];
        let mut futures = vec![];
        for proto_lane in self.proto_lanes.drain(..)
        {
            let future = proto_lane.evolve().boxed();
            futures.push(future);
        }

        let (lane, _ready_future_index, remaining_futures) = select_all(futures).await;

        let mut lane = lane?;

        lanes.push(lane);
        for future in remaining_futures
        {
          let lane = future.await?;
          lanes.push(lane);
        }

        unimplemented!();
/*        let kernel = self.kind.evolve()?;

        Ok(Arc::new(Star{
           shell: StarShell::new( lanes, kernel )
        }))

 */
    }
}

pub struct ProtoStarController
{
    command_tx: Sender<StarCommand>
}


#[derive(Clone)]
pub enum ProtoStarKernel
{
   Central,
   Mesh,
   Supervisor,
   Server,
   Gateway
}


impl ProtoStarKernel
{
    fn evolve(&self) -> Result<Box<dyn StarKernel>, Error>
    {
        Ok(Box::new(PlaceholderKernel::new()))
    }
}


pub struct PlaceholderKernel
{

}

impl PlaceholderKernel{
    pub fn new()->Self
    {
        PlaceholderKernel{}
    }
}

impl StarKernel for PlaceholderKernel
{

}


pub struct ProtoTunnel
{
    pub star: Option<StarKey>,
    pub tx: Sender<LaneGram>,
    pub rx: Receiver<LaneGram>,
}

impl ProtoTunnel
{

    pub async fn evolve(mut self) -> Result<Tunnel,Error>
    {
        self.tx.send(LaneGram::Proto(ProtoGram::StarLaneProtocolVersion(STARLANE_PROTOCOL_VERSION))).await;

        if let Option::Some(star)=self.star
        {
            self.tx.send(LaneGram::Proto(ProtoGram::ReportStarKey(star))).await;
        }

        // first we confirm that the version is as expected
        if let Option::Some(LaneGram::Proto(recv)) = self.rx.recv().await
        {
            match recv
            {
                ProtoGram::StarLaneProtocolVersion(version) if version == STARLANE_PROTOCOL_VERSION => {
                    // do nothing... we move onto the next step
                },
                ProtoGram::StarLaneProtocolVersion(version) => {
                    return Err(format!("wrong version: {}", version).into());
                },
                gram => {
                    return Err(format!("unexpected star gram: {} (expected to receive StarLaneProtocolVersion first)", gram).into());
                }
            }
        }
        else {
            return Err("disconnected".into());
        }

        if let Option::Some(LaneGram::Proto(recv)) = self.rx.recv().await
        {
            match recv
            {
                ProtoGram::ReportStarKey(remote_star_key) => {
                    let (signal_tx,_) = broadcast::channel(1);
                    return Ok(Tunnel{
                        remote_star: remote_star_key,
                        tx: self.tx,
                        rx: self.rx,
                        signal_tx: signal_tx
                    });
                }
                gram => { return Err(format!("unexpected star gram: {} (expected to receive ReportStarId next)", gram).into()); }
            };
        }
        else {
            return Err("disconnected!".into())
        }
    }
}

pub fn local_tunnels(high: StarKey, low:StarKey) ->(ProtoTunnel, ProtoTunnel)
{
    let (atx,arx) = mpsc::channel::<LaneGram>(32);
    let (btx,brx) = mpsc::channel::<LaneGram>(32);

    (ProtoTunnel {
        star: Option::Some(high),
        tx: atx,
        rx: brx
    },
     ProtoTunnel
    {
        star: Option::Some(low),
        tx: btx,
        rx: arx
    })
}
