use crate::machine::MachineSkel;
use crate::star::StarCall::LayerTraversalInjection;
use crate::star::{LayerInjectionRouter, StarSkel, StarState, StateApi, StateCall};
use crate::{PlatErr, Platform, RegistryApi};
use cosmic_api::command::command::common::{SetProperties, StateSrc};
use cosmic_api::config::config::bind::RouteSelector;
use cosmic_api::error::MsgErr;
use cosmic_api::id::id::{
    BaseKind, Kind, Layer, Point, Port, ToBaseKind, ToPoint, ToPort, TraversalLayer, Uuid,
};
use cosmic_api::id::{BaseSubKind, StarKey, Traversal, TraversalInjection};
use cosmic_api::log::{PointLogger, Tracker};
use cosmic_api::parse::model::Subst;
use cosmic_api::parse::route_attribute;
use cosmic_api::particle::particle::{Details, Status, Stub};
use cosmic_api::substance::substance::Substance;
use cosmic_api::sys::{Assign, AssignmentKind, Sys};
use cosmic_api::util::{log, ValuePattern};
use cosmic_api::wave::{
    Agent, Bounce, CoreBounce, DirectedCore, DirectedHandler, DirectedHandlerSelector,
    DirectedKind, DirectedProto, DirectedWave, Exchanger, InCtx, Ping, Pong, ProtoTransmitter,
    ProtoTransmitterBuilder, RecipientSelector, ReflectedCore, ReflectedWave, RootInCtx, Router,
    SetStrategy, SysMethod, UltraWave, Wave, WaveKind,
};
use cosmic_api::{Registration, RegistrationStrategy, State, HYPERUSER};
use dashmap::DashMap;
use futures::future::select_all;
use futures::FutureExt;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::Receiver;
use tokio::sync::watch::Ref;
use tokio::sync::{broadcast, mpsc, oneshot, watch, RwLock};

pub struct DriversBuilder<P>
where
    P: Platform,
{
    map: HashMap<Kind, Arc<dyn DriverFactory<P>>>,
}

impl<P> DriversBuilder<P>
where
    P: Platform,
{
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn kinds(&self) -> HashSet<Kind> {
        self.map.keys().cloned().into_iter().collect()
    }

    pub fn add(&mut self, factory: Arc<dyn DriverFactory<P>>) {
        self.map.insert(factory.kind(), factory);
    }

    pub fn build(
        self,
        skel: StarSkel<P>,
        call_tx: mpsc::Sender<DriversCall<P>>,
        call_rx: mpsc::Receiver<DriversCall<P>>,
        status_tx: watch::Sender<DriverStatus>,
        status_rx: watch::Receiver<DriverStatus>
    ) -> DriversApi<P> {
        let port = skel.point.push("drivers").unwrap().to_port();
        Drivers::new(port, skel.clone(), self.map, call_tx, call_rx, status_tx, status_rx )
    }
}

pub enum DriversCall<P>
where
    P: Platform,
{
    Init0,
    Init1,
    AddDriver {
        kind: Kind,
        driver: DriverApi<P>,
        rtn: oneshot::Sender<()>
    },
    Visit(Traversal<UltraWave>),
    Kinds(oneshot::Sender<Vec<Kind>>),
    Assign {
        assign: Assign,
        rtn: oneshot::Sender<Result<(), MsgErr>>,
    },
    Drivers(oneshot::Sender<HashMap<Kind, DriverApi<P>>>),
    Status {
        kind: Kind,
        rtn: oneshot::Sender<Result<DriverStatus, MsgErr>>,
    },
    StatusRx(oneshot::Sender<watch::Receiver<DriverStatus>>),
}

#[derive(Clone)]
pub struct DriversApi<P>
where
    P: Platform,
{
    call_tx: mpsc::Sender<DriversCall<P>>,
    status_rx: watch::Receiver<DriverStatus>,
}

impl<P> DriversApi<P>
where
    P: Platform,
{
    pub fn new(tx: mpsc::Sender<DriversCall<P>>, status_rx: watch::Receiver<DriverStatus>) -> Self {
        Self {
            call_tx: tx,
            status_rx,
        }
    }

    pub fn status(&self) -> DriverStatus {
        self.status_rx.borrow().clone()
    }

    pub async fn status_changed(&mut self) -> Result<DriverStatus, MsgErr> {
        self.status_rx.changed().await?;
        Ok(self.status())
    }

    pub async fn visit(&self, traversal: Traversal<UltraWave>) {
        self.call_tx.send(DriversCall::Visit(traversal)).await;
    }

    pub async fn kinds(&self) -> Result<Vec<Kind>, MsgErr> {
        let (rtn, mut rtn_rx) = oneshot::channel();
        self.call_tx.send(DriversCall::Kinds(rtn)).await;
        Ok(rtn_rx.await?)
    }

    pub async fn drivers(&self) -> Result<HashMap<Kind, DriverApi<P>>, MsgErr> {
        let (rtn, mut rtn_rx) = oneshot::channel();
        self.call_tx.send(DriversCall::Drivers(rtn)).await;
        Ok(rtn_rx.await?)
    }

    pub async fn init(&self) {
        self.call_tx.send(DriversCall::Init0).await;
    }

    pub async fn assign(&self, assign: Assign) -> Result<(), MsgErr> {
        println!("DriversApi ENTERING ASSIGN");
        let (rtn, rtn_rx) = oneshot::channel();
        self.call_tx.send(DriversCall::Assign { assign, rtn }).await;
        let rtn = Ok(rtn_rx.await??);
        println!("DriversApi RETURNING FROM ASSIGN");
        rtn
    }
}

#[derive(DirectedHandler)]
pub struct Drivers<P>
where
    P: Platform + 'static,
{
    port: Port,
    skel: StarSkel<P>,
    factories: HashMap<Kind, Arc<dyn DriverFactory<P>>>,
    drivers: HashMap<Kind, DriverApi<P>>,
    call_rx: mpsc::Receiver<DriversCall<P>>,
    call_tx: mpsc::Sender<DriversCall<P>>,
    statuses_rx: Arc<DashMap<Kind, watch::Receiver<DriverStatus>>>,
    status_tx: mpsc::Sender<DriverStatus>,
    status_rx: watch::Receiver<DriverStatus>,
    init: bool,
}

impl<P> Drivers<P>
where
    P: Platform + 'static,
{
    pub fn new(
        port: Port,
        skel: StarSkel<P>,
        factories: HashMap<Kind, Arc<dyn DriverFactory<P>>>,
        call_tx: mpsc::Sender<DriversCall<P>>,
        call_rx: mpsc::Receiver<DriversCall<P>>,
        watch_status_tx: watch::Sender<DriverStatus>,
        watch_status_rx: watch::Receiver<DriverStatus>
    ) -> DriversApi<P> {
        let statuses_rx = Arc::new(DashMap::new());
        let drivers = HashMap::new();
        let (mpsc_status_tx, mut mpsc_status_rx): (
            tokio::sync::mpsc::Sender<DriverStatus>,
            tokio::sync::mpsc::Receiver<DriverStatus>,
        ) = mpsc::channel(128);

        tokio::spawn(async move {
            while let Some(status) = mpsc_status_rx.recv().await {
                watch_status_tx.send(status.clone());
                if let DriverStatus::Fatal(_) = status {
                    break;
                }
            }
        });

        let mut drivers = Self {
            port,
            skel,
            drivers,
            call_rx,
            call_tx: call_tx.clone(),
            statuses_rx,
            factories,
            status_tx: mpsc_status_tx,
            status_rx: watch_status_rx.clone(),
            init: false,
        };

        drivers.start();

        DriversApi::new(call_tx, watch_status_rx)
    }

    fn start(mut self) {
        tokio::spawn(async move {
            while let Some(call) = self.call_rx.recv().await {
                match call {
                    DriversCall::Init0 => {
                        self.init0().await;
                    }
                    DriversCall::Init1 => {
                        self.init1().await;
                    }
                    DriversCall::AddDriver { kind, driver, rtn} => {
                        self.drivers.insert(kind, driver);
                        rtn.send(());
                    }
                    DriversCall::Visit(traversal) => {
                        self.visit(traversal).await;
                    }
                    DriversCall::Kinds(rtn) => {
                        rtn.send(self.kinds());
                    }
                    DriversCall::Assign { assign, rtn } => {
                        rtn.send(self.assign(assign).await).unwrap_or_default();
                    }
                    DriversCall::Drivers(rtn) => {
                        rtn.send(self.drivers.clone()).unwrap_or_default();
                    }
                    DriversCall::Status { kind, rtn } => match self.statuses_rx.get(&kind) {
                        None => {
                            rtn.send(Err(MsgErr::not_found()));
                        }
                        Some(status_rx) => {
                            rtn.send(Ok(status_rx.borrow().clone()));
                        }
                    },
                    DriversCall::StatusRx(rtn) => {
                        rtn.send(self.status_rx.clone());
                    }
                }
            }
        });
    }

    pub fn kinds(&self) -> Vec<Kind> {
        self.factories.keys().cloned().into_iter().collect()
    }
    pub async fn init0(&mut self) {

        let (status_tx, mut status_rx) = watch::channel(DriverStatus::Pending);
        self.statuses_rx.insert(Kind::Driver, status_rx.clone());

        let driver_driver_factory = Arc::new(DriverDriverFactory::new());
        self.create(Kind::Driver, driver_driver_factory, status_tx).await;

        // wait for DriverDriver to be ready
        let call_tx = self.call_tx.clone();
        tokio::spawn( async move {
            loop {
                if status_rx.borrow().clone() == DriverStatus::Ready {
                    break;
                }
                status_rx.changed().await.unwrap();
            }
            call_tx.send( DriversCall::Init1).await;
        });
    }

    pub async fn init1(&mut self) {
        let mut statuses_tx = HashMap::new();
        for kind in self.factories.keys() {
            let (status_tx, status_rx) = watch::channel(DriverStatus::Pending);
            statuses_tx.insert(kind.clone(), status_tx);
            self.statuses_rx.insert(kind.clone(), status_rx);
        }

        self.status_listen().await;

        for (kind, status_tx) in statuses_tx {
            let factory = self.factories.get(&kind).unwrap().clone();
            self.create(kind,factory, status_tx).await;
        }
    }

    async fn status_listen(&self) {
        let logger = self.skel.logger.clone();
        let status_tx = self.status_tx.clone();
        let statuses_rx = self.statuses_rx.clone();
        tokio::spawn(async move {
            loop {
                let mut inits = 0;
                let mut fatals = 0;
                let mut retries = 0;
                let mut readies = 0;

                if statuses_rx.is_empty() {
                    break;
                }

                for multi in statuses_rx.iter() {
                    let kind = multi.key();
                    let status_rx = multi.value();
                    match status_rx.borrow().clone() {
                        DriverStatus::Ready => {
                            readies = readies + 1;
                        }
                        DriverStatus::Retrying(msg) => {
                            logger.warn(format!("DRIVER RETRY: {} {}", kind.to_string(), msg));
                            retries = retries + 1;
                        }
                        DriverStatus::Fatal(msg) => {
                            logger.error(format!("DRIVER FATAL: {} {}", kind.to_string(), msg));
                            fatals = fatals + 1;
                        }
                        DriverStatus::Initializing => {
                            inits = inits + 1;
                        }
                        _ => {
                            break;
                        }
                    }
                }

                if readies == statuses_rx.len() {
                    status_tx.send(DriverStatus::Ready).await;
                } else if fatals > 0 {
                    status_tx.send(DriverStatus::Fatal(
                        "One or more Drivers have a Fatal condition".to_string(),
                    )).await;
                    break;
                } else if retries > 0 {
                    status_tx.send(DriverStatus::Fatal(
                        "One or more Drivers is Retrying initialization".to_string(),
                    )).await;
                } else if inits > 0 {
                    status_tx.send(DriverStatus::Initializing).await;
                } else {
                    status_tx.send(DriverStatus::Unknown).await;
                }

                for mut multi in statuses_rx.iter_mut() {
                    let status_rx = multi.value_mut();
                    let mut rx = vec![];
                    rx.push(status_rx.changed().boxed());
                    let (result, _, _) = select_all(rx).await;
                    if logger.result(result).is_err() {
                        break;
                    }
                }
            }
        });
    }

    async fn create( &self, kind: Kind, factory: Arc<dyn DriverFactory<P>>, status_tx: watch::Sender<DriverStatus>) {
        {
            let skel = self.skel.clone();
            let call_tx = self.call_tx.clone();
            let drivers_point = self.skel.point.push("drivers").unwrap();

            async fn register<P>(
                skel: &StarSkel<P>,
                point: &Point,
                logger: &PointLogger,
            ) -> Result<(), P::Err>
                where
                    P: Platform,
            {
                let registration = Registration {
                    point: point.clone(),
                    kind: Kind::Base(BaseSubKind::Drivers),
                    registry: Default::default(),
                    properties: Default::default(),
                    owner: HYPERUSER.clone(),
                    strategy: RegistrationStrategy::Overwrite,
                };

                skel.registry.register(&registration).await?;
                skel.registry.assign(&point, &skel.point).await?;
                skel.registry
                    .set_status(&point, &Status::Init)
                    .await?;
                skel.api.create_states(point.clone()).await;
                Ok(())
            }
            let point = drivers_point.push(kind.as_point_segments()).unwrap();
            let logger = self.skel.logger.point(point.clone());
            let status_rx = status_tx.subscribe();

            {
                let logger = logger.point(point.clone());
                let kind = kind.clone();
                let mut status_rx = status_rx.clone();
                tokio::spawn(async move {
                    loop {
                        let status = status_rx.borrow().clone();
                        logger.info(format!("{} {}", kind.to_string(), status.to_string() ));
                        status_rx.changed().await.unwrap();
                    }
                });
            }

            match logger.result(register(&skel, &point, &logger).await) {
                Ok(_) => {}
                Err(err) => {
                    status_tx.send(DriverStatus::Fatal(
                        "Driver registration failed".to_string(),
                    ));
                    return;
                }
            }

            let router = Arc::new(LayerInjectionRouter::new(
                skel.clone(),
                point.clone().to_port().with_layer(Layer::Guest),
            ));
            let mut transmitter = ProtoTransmitterBuilder::new(router, skel.exchanger.clone());
            transmitter.from =
                SetStrategy::Override(point.clone().to_port().with_layer(Layer::Core));
            let transmitter = transmitter.build();

            let (shell_tx, shell_rx) = mpsc::channel(1024);
            let driver_skel = DriverSkel::new(
                kind.clone(),
                point.clone(),
                transmitter,
                logger.clone(),
                status_tx,
            );

            {
                let skel = self.skel.clone();
                let call_tx = call_tx.clone();
                let logger = logger.clone();
                let router = Arc::new(self.skel.gravity_router.clone());
                let mut transmitter =
                    ProtoTransmitterBuilder::new(router, self.skel.exchanger.clone());
                transmitter.from = SetStrategy::Override(
                    self.skel.point.clone().to_port().with_layer(Layer::Gravity),
                );
                transmitter.agent = SetStrategy::Override(Agent::HyperUser);
                let ctx = DriverInitCtx::new(transmitter.build());

                tokio::spawn(async move {
                    let driver = logger.result(factory.init(driver_skel.clone(), &ctx).await);
                    match driver {
                        Ok(driver) => {
                            let runner = DriverRunner::new(
                                driver_skel.clone(),
                                skel.clone(),
                                driver,
                                shell_tx,
                                shell_rx,
                                status_rx.clone(),
                            );
                            let driver = DriverApi::new(runner.clone(), factory.kind());
                            let (rtn,rtn_rx) = oneshot::channel();
                            call_tx
                                .send(DriversCall::AddDriver { kind, driver, rtn })
                                .await
                                .unwrap_or_default();
                            rtn_rx.await;
                            runner.send( DriverRunnerCall::OnAdded ).await;
                        }
                        Err(err) => {
                            logger.error(err.to_string());
                            driver_skel.status_tx.send(DriverStatus::Fatal("Driver Factory creation error".to_string())).await;
                        }
                    }
                });
            }
        }
    }
}

impl<P> Drivers<P>
where
    P: Platform,
{
    pub async fn assign(&self, assign: Assign) -> Result<(), MsgErr> {
        let driver = self
            .drivers
            .get(&assign.details.stub.kind)
            .ok_or::<MsgErr>(
                format!(
                    "kind not supported by these Drivers: {}",
                    assign.details.stub.kind.to_string()
                )
                .into(),
            )?;
        driver.assign(assign).await
    }

    pub async fn handle(&self, wave: DirectedWave) -> Result<ReflectedCore, MsgErr> {
        let record = self
            .skel
            .registry
            .locate(&wave.to().single_or()?.point)
            .await
            .map_err(|e| e.to_cosmic_err())?;
        let driver = self
            .drivers
            .get(&record.details.stub.kind)
            .ok_or::<MsgErr>("do not handle this kind of driver".into())?;
        driver.handle(wave).await
    }

    /*
    pub async fn sys(&self, ctx: InCtx<'_, Sys>) -> Result<ReflectedCore, MsgErr> {
        if let Sys::Assign(assign) = &ctx.input {
            match self.drivers.get(&assign.details.stub.kind) {
                None => Err(format!(
                    "do not have driver for Kind: <{}>",
                    assign.details.stub.kind.to_string()
                )
                .into()),
                Some(driver) => {
                    let ctx = ctx.push_input_ref( assign );
                    let state = tokio::time::timeout(
                        Duration::from_secs(self.skel.machine.timeouts.high),
                        driver.assign(ctx).await,
                    )
                    .await??;
                   Ok(ctx.wave().core.ok())
                }
            }
        } else {
            Err(MsgErr::bad_request())
        }
    }

     */

    async fn start_outer_traversal(&self, traversal: Traversal<UltraWave>) {
        let traverse_to_next_tx = self.skel.traverse_to_next_tx.clone();
        tokio::spawn(async move {
            traverse_to_next_tx.send(traversal).await;
        });
    }

    async fn start_inner_traversal(&self, traversal: Traversal<UltraWave>) {}

    pub async fn visit(&self, traversal: Traversal<UltraWave>) {
        if traversal.dir.is_core() {
            match self.drivers.get(&traversal.record.details.stub.kind) {
                None => {
                    traversal.logger.warn(format!(
                        "star does not have a driver for Kind <{}>",
                        traversal.record.details.stub.kind.to_string()
                    ));
                }
                Some(driver) => {
                    let driver = driver.clone();
                    tokio::spawn(async move {
                        driver.traversal(traversal).await;
                    });
                }
            }
        } else {
            self.start_outer_traversal(traversal).await;
        }
    }
}

#[derive(Clone)]
pub struct DriverApi<P>
where
    P: Platform,
{
    pub call_tx: mpsc::Sender<DriverRunnerCall<P>>,
    pub kind: Kind,
}

impl<P> DriverApi<P>
where
    P: Platform,
{
    pub fn new(tx: mpsc::Sender<DriverRunnerCall<P>>, kind: Kind) -> Self {
        Self { call_tx: tx, kind }
    }


    pub fn on_added(&self) {
        self.call_tx
            .try_send(DriverRunnerCall::OnAdded);
    }


    pub async fn assign(&self, assign: Assign) -> Result<(), MsgErr> {
        let (rtn, rtn_rx) = oneshot::channel();
        self.call_tx
            .send(DriverRunnerCall::Assign { assign, rtn })
            .await;
        Ok(rtn_rx.await??)
    }

    pub async fn traversal(&self, traversal: Traversal<UltraWave>) {
        self.call_tx
            .send(DriverRunnerCall::Traversal(traversal))
            .await;
    }

    pub async fn handle(&self, wave: DirectedWave) -> Result<ReflectedCore, MsgErr> {
        let (tx, mut rx) = oneshot::channel();
        self.call_tx
            .send(DriverRunnerCall::Handle { wave, tx })
            .await;
        tokio::time::timeout(Duration::from_secs(30), rx).await??
    }
}
/*
fn create_driver<P>(
    factory: Box<dyn DriverFactory<P>>,
    drivers_port: Port,
    skel: StarSkel<P>,
) -> Result<DriverApi, MsgErr>
where
    P: Platform + 'static,
{
    let point = drivers_port
        .point
        .push(factory.kind().as_point_segments())?;
    let (shell_tx, shell_rx) = mpsc::channel(1024);
    let (tx, mut rx) = mpsc::channel(1024);
    {
        let shell_tx = shell_tx.clone();
        tokio::spawn(async move {
            while let Some(call) = rx.recv().await {
                match call {
                    DriverShellRequest::Ex { point, tx } => {
                        let call = DriverShellCall::Item { point, tx };
                        shell_tx.send(call).await;
                    }
                    DriverShellRequest::Assign { assign, rtn } => {
                        let call = DriverShellCall::Assign { assign, rtn };
                        shell_tx.send(call).await;
                    }
                }
            }
        });
    }
    let router = Arc::new(LayerInjectionRouter::new(
        skel.clone(),
        point.clone().to_port().with_layer(Layer::Guest),
    ));
    let (driver_skel,status_tx,status_ctx_tx) = DriverSkel::new(point.clone(), router, tx, skel.clone());
    let driver = factory.create(driver_skel, status_tx);
    let state = skel.state.api().with_layer(Layer::Core);
    let shell = DriverShell::new(point, skel.clone(), driver, state, shell_tx, shell_rx);
    let api = DriverApi::new(shell, factory.kind());
    Ok(api)
}

 */

pub enum DriverRunnerCall<P>
where
    P: Platform,
{
    Traversal(Traversal<UltraWave>),
    Handle {
        wave: DirectedWave,
        tx: oneshot::Sender<Result<ReflectedCore, MsgErr>>,
    },
    Item {
        point: Point,
        tx: oneshot::Sender<Result<Box<dyn ItemHandler<P>>, P::Err>>,
    },
    Assign {
        assign: Assign,
        rtn: oneshot::Sender<Result<(), MsgErr>>,
    },
    OnAdded
}

pub struct ItemShell<P>
where
    P: Platform + 'static,
{
    pub port: Port,
    pub skel: StarSkel<P>,
    pub state: Option<Arc<RwLock<dyn State>>>,
    pub item: Box<dyn ItemHandler<P>>,
    pub router: Arc<dyn Router>,
}

#[async_trait]
impl<P> TraversalLayer for ItemShell<P>
where
    P: Platform,
{
    fn port(&self) -> &cosmic_api::id::id::Port {
        &self.port
    }

    async fn deliver_directed(&self, direct: Traversal<DirectedWave>) {
        self.skel
            .logger
            .track(&direct, || Tracker::new("core:outer", "DeliverDirected"));
        let logger = self
            .skel
            .logger
            .point(self.port().clone().to_point())
            .span();
        let mut transmitter =
            ProtoTransmitterBuilder::new(self.router.clone(), self.skel.exchanger.clone());
        transmitter.from = SetStrategy::Override(self.port.clone());
        let transmitter = transmitter.build();
        let to = direct.to().clone().unwrap_single();
        let reflection = direct.reflection();
        let ctx = RootInCtx::new(direct.payload, to, logger, transmitter);
        match self.item.handle(ctx).await {
            CoreBounce::Absorbed => {}
            CoreBounce::Reflected(reflected) => {
                let wave = reflection.unwrap().make(reflected, self.port.clone());
                let wave = wave.to_ultra();
                #[cfg(test)]
                self.skel
                    .diagnostic_interceptors
                    .reflected_endpoint
                    .send(wave.clone());
                self.inject(wave).await;
            }
        }
    }

    async fn deliver_reflected(&self, reflect: Traversal<ReflectedWave>) {
        self.exchanger().reflected(reflect.payload).await;
    }

    async fn traverse_next(&self, traversal: Traversal<UltraWave>) {
        self.skel.traverse_to_next_tx.send(traversal).await;
    }

    async fn inject(&self, wave: UltraWave) {
        let inject = TraversalInjection::new(self.port().clone().with_layer(Layer::Guest), wave);
        self.skel.inject_tx.send(inject).await;
    }

    fn exchanger(&self) -> &Exchanger {
        &self.skel.exchanger
    }
}

#[derive(DirectedHandler)]
pub struct DriverRunner<P>
where
    P: Platform + 'static,
{
    skel: DriverSkel<P>,
    star_skel: StarSkel<P>,
    call_tx: mpsc::Sender<DriverRunnerCall<P>>,
    call_rx: mpsc::Receiver<DriverRunnerCall<P>>,
    driver: Box<dyn Driver<P>>,
    router: LayerInjectionRouter<P>,
    logger: PointLogger,
    status_rx: watch::Receiver<DriverStatus>,
}

#[routes]
impl<P> DriverRunner<P>
where
    P: Platform + 'static,
{
    pub fn new(
        skel: DriverSkel<P>,
        star_skel: StarSkel<P>,
        driver: Box<dyn Driver<P>>,
        call_tx: mpsc::Sender<DriverRunnerCall<P>>,
        call_rx: mpsc::Receiver<DriverRunnerCall<P>>,
        status_rx: watch::Receiver<DriverStatus>,
    ) -> mpsc::Sender<DriverRunnerCall<P>> {
        let logger = star_skel.logger.point(skel.point.clone());
        let router = LayerInjectionRouter::new(
            star_skel.clone(),
            skel.point.clone().to_port().with_layer(Layer::Guest),
        );

        let driver = Self {
            skel,
            star_skel: star_skel,
            call_tx: call_tx.clone(),
            call_rx: call_rx,
            driver,
            router,
            logger,
            status_rx,
        };

        driver.start();

        call_tx
    }

    fn start(mut self) {
        tokio::spawn(async move {
            while let Some(call) = self.call_rx.recv().await {
                match call {
                    DriverRunnerCall::OnAdded => {
                        let router = Arc::new(LayerInjectionRouter::new( self.star_skel.clone(), self.skel.point.clone().to_port().with_layer(Layer::Core)));
                        let transmitter = ProtoTransmitter::new( router, self.star_skel.exchanger.clone() );
                        let ctx = DriverInitCtx::new(transmitter);
                        self.driver.init(self.skel.clone(), ctx ).await;
                    }
                    DriverRunnerCall::Traversal(traversal) => {
                        self.traverse(traversal).await;
                    }
                    DriverRunnerCall::Handle { wave, tx } => {
                        self.logger
                            .track(&wave, || Tracker::new("driver:shell", "Handle"));
                        let port = wave.to().clone().unwrap_single();
                        let logger = self.star_skel.logger.point(port.clone().to_point()).span();
                        let router = Arc::new(self.router.clone());
                        let transmitter =
                            ProtoTransmitter::new(router, self.star_skel.exchanger.clone());
                        let ctx = RootInCtx::new(wave, port.clone(), logger, transmitter);
                        match self.handle(ctx).await {
                            CoreBounce::Absorbed => {
                                tx.send(Err(MsgErr::server_error()));
                            }
                            CoreBounce::Reflected(reflect) => {
                                tx.send(Ok(reflect));
                            }
                        }
                    }
                    DriverRunnerCall::Item { point, tx } => {
                        tx.send(self.driver.item(&point).await);
                    }
                    DriverRunnerCall::Assign { assign, rtn } => {
                        rtn.send(self.driver.assign(assign).await);
                    }
                }
            }
        });
    }

    async fn traverse(&self, traversal: Traversal<UltraWave>) -> Result<(), P::Err> {
        let core = self.item(&traversal.to.point).await?;
        if traversal.is_directed() {
            core.deliver_directed(traversal.unwrap_directed()).await;
        } else {
            core.deliver_reflected(traversal.unwrap_reflected()).await;
        }
        Ok(())
    }

    async fn item(&self, point: &Point) -> Result<ItemShell<P>, P::Err> {
        let port = point.clone().to_port().with_layer(Layer::Core);
        let (tx, mut rx) = oneshot::channel();
        self.star_skel
            .state
            .states_tx()
            .send(StateCall::Get {
                port: port.clone(),
                tx,
            })
            .await;
        let state = rx.await??;
        Ok(ItemShell {
            port: port.clone(),
            skel: self.star_skel.clone(),
            state: state.clone(),
            item: self.driver.item(point).await?,
            router: Arc::new(self.router.clone().with(port)),
        })
    }

    #[route("Sys<Assign>")]
    async fn assign(&self, ctx: InCtx<'_, Sys>) -> Result<ReflectedCore, MsgErr> {
        match ctx.input {
            Sys::Assign(assign) => {
                self.driver.assign(assign.clone()).await?;

                Ok(ReflectedCore::ok_body(Substance::Empty))
            }
            _ => Err(MsgErr::bad_request()),
        }
    }


}

pub struct DriverInitCtx {
    pub transmitter: ProtoTransmitter,
}

impl DriverInitCtx {
    pub fn new(transmitter: ProtoTransmitter) -> Self {
        Self { transmitter }
    }
}

#[derive(Clone)]
pub struct DriverSkel<P>
where
    P: Platform,
{
    pub kind: Kind,
    pub point: Point,
    pub logger: PointLogger,
    pub status_rx: watch::Receiver<DriverStatus>,
    pub status_tx: mpsc::Sender<DriverStatus>,
    pub phantom: PhantomData<P>,
}

impl<P> DriverSkel<P>
where
    P: Platform,
{
    pub fn status(&self) -> DriverStatus {
        self.status_rx.borrow().clone()
    }

    pub fn new(
        kind: Kind,
        point: Point,
        transmitter: ProtoTransmitter,
        logger: PointLogger,
        status_tx: watch::Sender<DriverStatus>,
    ) -> Self {
        let (mpsc_status_tx, mut mpsc_status_rx): (
            tokio::sync::mpsc::Sender<DriverStatus>,
            tokio::sync::mpsc::Receiver<DriverStatus>,
        ) = mpsc::channel(128);

        let watch_status_rx = status_tx.subscribe();
        tokio::spawn(async move {
            while let Some(status) = mpsc_status_rx.recv().await {
                status_tx.send(status.clone());
                if let DriverStatus::Fatal(_) = status {
                    break;
                }
            }
        });

        Self {
            kind,
            point,
            logger,
            status_tx: mpsc_status_tx,
            status_rx: watch_status_rx,
            phantom: Default::default(),
        }
    }
}

#[async_trait]
pub trait DriverFactory<P>: Send + Sync
where
    P: Platform,
{
    fn kind(&self) -> Kind;

    async fn init(
        &self,
        skel: DriverSkel<P>,
        ctx: &DriverInitCtx,
    ) -> Result<Box<dyn Driver<P>>, P::Err>;

    fn properties(&self) -> SetProperties {
        SetProperties::default()
    }
}

#[async_trait]
pub trait Driver<P>: DirectedHandler + Send + Sync
where
    P: Platform,
{
    fn kind(&self) -> Kind;

    async fn init(&self, skel: DriverSkel<P>, ctx: DriverInitCtx ) {
        skel.logger.result(skel.status_tx.send( DriverStatus::Ready ).await).unwrap_or_default();
    }

    async fn item(&self, point: &Point) -> Result<Box<dyn ItemHandler<P>>, P::Err>;
    async fn assign(&self, assign: Assign) -> Result<(), MsgErr>;
}

pub trait States: Sync + Sync
where
    Self::ItemState: ItemState,
{
    type ItemState;
    fn new() -> Self;

    fn create(assign: Assign) -> Arc<RwLock<Self::ItemState>>;
    fn get(point: &Point) -> Option<&Arc<RwLock<Self::ItemState>>>;
    fn remove(point: &Point) -> Option<Arc<RwLock<Self::ItemState>>>;
}

#[derive(Clone, Eq, PartialEq, Hash, strum_macros::Display)]
pub enum DriverStatus {
    Unknown,
    Pending,
    Initializing,
    Ready,
    Retrying(String),
    Fatal(String),
}

impl<E> From<Result<DriverStatus, E>> for DriverStatus
where
    E: ToString,
{
    fn from(result: Result<DriverStatus, E>) -> Self {
        match result {
            Ok(status) => status,
            Err(e) => DriverStatus::Fatal(e.to_string()),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct DriverStatusEvent {
    pub driver: Point,
    pub status: DriverStatus,
}

pub trait ItemState: Send + Sync {}

pub trait ItemHandler<P>: DirectedHandler + Send + Sync
where
    P: Platform,
{
}

pub trait Item<P>: ItemHandler<P> + Send + Sync
where
    P: Platform,
{
    type Skel;
    type Ctx;
    type State;

    fn restore(skel: Self::Skel, ctx: Self::Ctx, state: Self::State) -> Self;
}

#[derive(Clone)]
pub struct ItemSkel<P>
where
    P: Platform,
{
    pub point: Point,
    pub transmitter: ProtoTransmitter,
    phantom: PhantomData<P>,
}

impl<P> ItemSkel<P>
where
    P: Platform,
{
    pub fn new(point: Point, transmitter: ProtoTransmitter) -> Self {
        Self {
            point,
            transmitter,
            phantom: Default::default(),
        }
    }
}

pub struct DriverDriverFactory {}

impl DriverDriverFactory {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl<P> DriverFactory<P> for DriverDriverFactory
where
    P: Platform,
{
    fn kind(&self) -> Kind {
        Kind::Driver
    }

    async fn init(
        &self,
        skel: DriverSkel<P>,
        ctx: &DriverInitCtx,
    ) -> Result<Box<dyn Driver<P>>, P::Err> {
        Ok(Box::new(DriverDriver::new(skel).await?))
    }
}

#[derive(DirectedHandler)]
pub struct DriverDriver<P>
where
    P: Platform,
{
    skel: DriverSkel<P>,
}

#[routes]
impl<P> DriverDriver<P>
where
    P: Platform,
{
    async fn new(skel: DriverSkel<P>) -> Result<Self, P::Err> {
        Ok(Self { skel })
    }
}

#[async_trait]
impl<P> Driver<P> for DriverDriver<P>
where
    P: Platform,
{
    fn kind(&self) -> Kind {
        Kind::Driver
    }

    async fn item(&self, point: &Point) -> Result<Box<dyn ItemHandler<P>>, P::Err> {
        todo!()
    }

    async fn assign(&self, assign: Assign) -> Result<(), MsgErr> {
        Ok(())
    }
}

#[derive(DirectedHandler)]
pub struct DriverCore<P>
where
    P: Platform,
{
    skel: ItemSkel<P>,
}

#[routes]
impl<P> DriverCore<P>
where
    P: Platform,
{
    pub fn new(skel: ItemSkel<P>) -> Self {
        Self { skel }
    }
}

impl<P> ItemHandler<P> for DriverCore<P> where P: Platform {}

impl<P> Item<P> for DriverCore<P>
where
    P: Platform,
{
    type Skel = ItemSkel<P>;
    type Ctx = ();
    type State = ();

    fn restore(skel: Self::Skel, ctx: Self::Ctx, state: Self::State) -> Self {
        Self { skel }
    }
}
