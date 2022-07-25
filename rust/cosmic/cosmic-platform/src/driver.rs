use crate::machine::MachineSkel;
use crate::star::StarCall::LayerTraversalInjection;
use crate::star::{LayerInjectionRouter, StarSkel, StarState, StateApi, StateCall};
use crate::{PlatErr, Platform, RegistryApi};
use cosmic_api::config::config::bind::RouteSelector;
use cosmic_api::error::MsgErr;
use cosmic_api::id::id::{Kind, Layer, Point, Port, ToPoint, ToPort, TraversalLayer, Uuid};
use cosmic_api::id::{StarKey, Traversal, TraversalInjection};
use cosmic_api::log::PointLogger;
use cosmic_api::parse::model::Subst;
use cosmic_api::parse::route_attribute;
use cosmic_api::particle::particle::Status;
use cosmic_api::substance::substance::Substance;
use cosmic_api::sys::{Assign, Sys};
use cosmic_api::util::ValuePattern;
use cosmic_api::wave::{
    Bounce, CoreBounce, DirectedHandler, DirectedHandlerSelector, DirectedKind, DirectedProto,
    DirectedWave, Exchanger, InCtx, Ping, Pong, ProtoTransmitter, ProtoTransmitterBuilder,
    RecipientSelector, ReflectedCore, ReflectedWave, RootInCtx, Router, SetStrategy, SysMethod,
    UltraWave, Wave, WaveKind,
};
use cosmic_api::State;
use cosmic_driver::{
    Core, Driver, DriverFactory, DriverLifecycleCall, DriverShellRequest, DriverSkel, DriverStatus,
    DriverStatusEvent,
};
use dashmap::DashMap;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::sync::{broadcast, mpsc, oneshot, RwLock};

pub enum DriversCall<P>
where
    P: Platform,
{
    Init(oneshot::Sender<Result<Status, P::Err>>),
    Visit(Traversal<UltraWave>),
    Kinds(oneshot::Sender<Vec<Kind>>),
    Assign {
        assign: Assign,
        rtn: oneshot::Sender<Result<(), MsgErr>>,
    },
}

#[derive(Clone)]
pub struct DriversApi<P>
where
    P: Platform,
{
    tx: mpsc::Sender<DriversCall<P>>,
}

impl<P> DriversApi<P>
where
    P: Platform,
{
    pub fn new(tx: mpsc::Sender<DriversCall<P>>) -> Self {
        Self { tx }
    }

    pub async fn visit(&self, traversal: Traversal<UltraWave>) {
        self.tx.send(DriversCall::Visit(traversal)).await;
    }

    pub async fn kinds(&self) -> Result<Vec<Kind>, MsgErr> {
        let (rtn, mut rtn_rx) = oneshot::channel();
        self.tx.send(DriversCall::Kinds(rtn)).await;
        Ok(rtn_rx.await?)
    }

    pub async fn init(&self) -> Result<Status, P::Err>
    where
        <P as Platform>::Err: From<tokio::sync::oneshot::error::RecvError>,
    {
        let (rtn, mut rtn_rx) = oneshot::channel();
        self.tx.send(DriversCall::Init(rtn)).await;
        rtn_rx.await?
    }
    pub async fn assign(&self, assign: Assign) -> Result<(), MsgErr> {
        let (rtn, rtn_rx) = oneshot::channel();
        self.tx.send(DriversCall::Assign { assign, rtn }).await;
        Ok(rtn_rx.await??)
    }
}

#[derive(DirectedHandler)]
pub struct Drivers<P>
where
    P: Platform + 'static,
{
    port: Port,
    skel: StarSkel<P>,
    drivers: HashMap<Kind, DriverApi>,
    rx: mpsc::Receiver<DriversCall<P>>,
}

impl<P> Drivers<P>
where
    P: Platform + 'static,
{
    pub fn new(
        port: Port,
        skel: StarSkel<P>,
        drivers: HashMap<Kind, DriverApi>,
        tx: mpsc::Sender<DriversCall<P>>,
        rx: mpsc::Receiver<DriversCall<P>>,
    ) -> DriversApi<P> {
        let mut drivers = Self {
            port,
            skel,
            drivers,
            rx,
        };

        drivers.start();

        DriversApi::new(tx)
    }

    fn start(mut self) {
        tokio::spawn(async move {
            while let Some(call) = self.rx.recv().await {
                match call {
                    DriversCall::Init(rtn) => {
                        rtn.send(self.init().await);
                    }
                    DriversCall::Visit(traversal) => {
                        self.visit(traversal).await;
                    }
                    DriversCall::Kinds(rtn) => {
                        rtn.send(self.kinds());
                    }
                    DriversCall::Assign { assign, rtn } => {
                        rtn.send(self.assign(assign).await);
                    }
                }
            }
        });
    }

    pub fn kinds(&self) -> Vec<Kind> {
        let mut rtn = vec![];
        for (kind, _) in &self.drivers {
            rtn.push(kind.clone())
        }
        rtn
    }

    pub async fn init(&self) -> Result<Status, P::Err> {
        let mut errs = vec![];
        for driver in self.drivers.values() {
            let status = driver.status().await?;
            if status != DriverStatus::Ready && status != DriverStatus::Initializing {
                match driver.lifecycle(DriverLifecycleCall::Init).await {
                    Ok(status) => {
                        if status != DriverStatus::Ready {
                            errs.push(MsgErr::server_error());
                        }
                    }
                    Err(err) => {
                        errs.push(err);
                    }
                }
            }
        }

        if !errs.is_empty() {
            // need to fold these errors into one
            Err(MsgErr::server_error().into())
        } else {
            Ok(Status::Ready)
        }
    }

    /*
    pub fn add(&mut self, factory: Box<dyn DriverFactory>) -> Result<(), MsgErr> {
        let kind = factory.kind().clone();
        let api = create_driver(factory, self.port.clone(), self.skel.clone())?;
        self.drivers.insert(kind, api);
        Ok(())

    }
     */
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
        self.skel.traverse_to_next_tx.send(traversal).await;
    }

    async fn start_inner_traversal(&self, traversal: Traversal<UltraWave>) {}

    pub async fn visit(&self, traversal: Traversal<UltraWave>) {
        println!("Visiting Drivers...");
        if traversal.dir.is_core() {
            match self.drivers.get(&traversal.record.details.stub.kind) {
                None => {
                    traversal.logger.warn(format!(
                        "star does not have a driver for Kind <{}>",
                        traversal.record.details.stub.kind.to_string()
                    ));
                }
                Some(driver) => {
                    driver.traversal(traversal).await;
                }
            }
        } else {
            self.start_outer_traversal(traversal).await;
        }
    }
}

#[derive(Clone)]
pub struct DriverApi {
    pub tx: mpsc::Sender<DriverShellCall>,
    pub kind: Kind,
}

impl DriverApi {
    pub fn new(tx: mpsc::Sender<DriverShellCall>, kind: Kind) -> Self {
        Self { tx, kind }
    }

    pub async fn assign(&self, assign: Assign) -> Result<(), MsgErr> {
        let (rtn, rtn_rx) = oneshot::channel();
        self.tx.send(DriverShellCall::Assign { assign, rtn }).await;
        Ok(rtn_rx.await??)
    }

    pub async fn status(&self) -> Result<DriverStatus, MsgErr> {
        let (tx, mut rx) = oneshot::channel();
        self.tx.send(DriverShellCall::Status(tx)).await;
        Ok(tokio::time::timeout(Duration::from_secs(60), rx).await??)
    }

    pub async fn lifecycle(&self, call: DriverLifecycleCall) -> Result<DriverStatus, MsgErr> {
        let (tx, mut rx) = oneshot::channel();
        self.tx
            .send(DriverShellCall::LifecycleCall { call, tx })
            .await;

        tokio::time::timeout(Duration::from_secs(5 * 60), rx).await??
    }

    pub async fn traversal(&self, traversal: Traversal<UltraWave>) {
        println!("sending along driver: {}", self.kind.to_string());
        self.tx.send(DriverShellCall::Traversal(traversal)).await;
    }

    pub async fn handle(&self, wave: DirectedWave) -> Result<ReflectedCore, MsgErr> {
        let (tx, mut rx) = oneshot::channel();
        self.tx.send(DriverShellCall::Handle { wave, tx }).await;
        tokio::time::timeout(Duration::from_secs(30), rx).await??
    }
}

pub struct DriversBuilder {
    pub factories: HashMap<Kind, Box<dyn DriverFactory>>,
    pub logger: Option<PointLogger>,
}

impl DriversBuilder {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
            logger: None,
        }
    }

    pub fn kinds(&self) -> HashSet<Kind> {
        let mut rtn = HashSet::new();
        for kind in self.factories.keys() {
            rtn.insert(kind.clone());
        }
        rtn
    }

    pub fn add(&mut self, factory: Box<dyn DriverFactory>) {
        self.factories.insert(factory.kind().clone(), factory);
    }

    pub fn logger(&mut self, logger: PointLogger) {
        self.logger.replace(logger);
    }

    pub fn build<P>(
        self,
        drivers_port: Port,
        skel: StarSkel<P>,
        drivers_tx: mpsc::Sender<DriversCall<P>>,
        drivers_rx: mpsc::Receiver<DriversCall<P>>,
    ) -> Result<DriversApi<P>, MsgErr>
    where
        P: Platform + 'static,
    {
        if self.logger.is_none() {
            return Err("expected point logger to be set".into());
        }
        let mut drivers = HashMap::new();
        for (_, factory) in self.factories {
            let kind = factory.kind().clone();
            let api = create_driver(factory, drivers_port.clone(), skel.clone())?;
            drivers.insert(kind, api);
        }
        Ok(Drivers::new(
            drivers_port,
            skel,
            drivers,
            drivers_tx,
            drivers_rx,
        ))
    }
}

fn create_driver<P>(
    factory: Box<dyn DriverFactory>,
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
                        let call = DriverShellCall::Ex { point, tx };
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
        point.clone().to_port().with_layer(Layer::Core),
    ));
    let driver_skel = DriverSkel::new(point.clone(), router, tx);
    let core = factory.create(driver_skel);
    let state = skel.state.api().with_layer(Layer::Core);
    let shell = DriverShell::new(point, skel.clone(), core, state, shell_tx, shell_rx);
    let api = DriverApi::new(shell, factory.kind());
    Ok(api)
}

pub enum DriverShellCall {
    LifecycleCall {
        call: DriverLifecycleCall,
        tx: oneshot::Sender<Result<DriverStatus, MsgErr>>,
    },
    Status(oneshot::Sender<DriverStatus>),
    Traversal(Traversal<UltraWave>),
    Handle {
        wave: DirectedWave,
        tx: oneshot::Sender<Result<ReflectedCore, MsgErr>>,
    },
    Ex {
        point: Point,
        tx: oneshot::Sender<Result<Box<dyn Core>, MsgErr>>,
    },
    Assign {
        assign: Assign,
        rtn: oneshot::Sender<Result<(), MsgErr>>,
    },
}

pub struct OuterCore<P>
where
    P: Platform + 'static,
{
    pub port: Port,
    pub skel: StarSkel<P>,
    pub state: Option<Arc<RwLock<dyn State>>>,
    pub ex: Box<dyn Core>,
    pub router: Arc<dyn Router>,
}

#[async_trait]
impl<P> TraversalLayer for OuterCore<P>
where
    P: Platform,
{
    fn port(&self) -> &cosmic_api::id::id::Port {
        &self.port
    }

    async fn deliver_directed(&self, direct: Traversal<DirectedWave>) {
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
        match self.ex.handle(ctx).await {
            CoreBounce::Absorbed => {
                println!("---> ABSORBED <----");
            }
            CoreBounce::Reflected(reflected) => {
                let wave = reflection.unwrap().make(reflected, self.port.clone());
                let wave = wave.to_ultra();
                println!("---> RELFECTED <-----");
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
        let inject = TraversalInjection::new(self.port().clone(), wave);
        self.skel.inject_tx.send(inject).await;
    }

    fn exchanger(&self) -> &Exchanger {
        &self.skel.exchanger
    }
}

#[derive(DirectedHandler)]
pub struct DriverShell<P>
where
    P: Platform + 'static,
{
    point: Point,
    skel: StarSkel<P>,
    status: DriverStatus,
    tx: mpsc::Sender<DriverShellCall>,
    rx: mpsc::Receiver<DriverShellCall>,
    state: StateApi,
    driver: Box<dyn Driver>,
    router: LayerInjectionRouter<P>,
    logger: PointLogger,
}

#[routes]
impl<P> DriverShell<P>
where
    P: Platform + 'static,
{
    pub fn new(
        point: Point,
        skel: StarSkel<P>,
        driver: Box<dyn Driver>,
        states: StateApi,
        tx: mpsc::Sender<DriverShellCall>,
        rx: mpsc::Receiver<DriverShellCall>,
    ) -> mpsc::Sender<DriverShellCall> {
        let logger = skel.logger.point(point.clone());
        let router = LayerInjectionRouter::new(
            skel.clone(),
            point.clone().to_port().with_layer(Layer::Driver),
        );

        let driver = Self {
            point,
            skel,
            status: DriverStatus::Started,
            tx: tx.clone(),
            rx,
            state: states,
            driver,
            router,
            logger,
        };

        driver.start();

        tx
    }

    fn start(mut self) {
        tokio::spawn(async move {
            while let Some(call) = self.rx.recv().await {
                println!("received Driver Shell Call!");
                match call {
                    DriverShellCall::LifecycleCall { call, tx } => {
                        let result = self.lifecycle(call).await;
                        match result {
                            Ok(status) => {
                                self.status = status.clone();
                                tx.send(Ok(status));
                            }
                            Err(err) => {
                                self.status = DriverStatus::Unknown;
                                tx.send(Err(err));
                            }
                        }
                    }
                    DriverShellCall::Status(tx) => {
                        tx.send(self.status.clone());
                    }
                    DriverShellCall::Traversal(traversal) => {
                        println!("Driver Shell Traversal:  ");
                        self.traverse(traversal).await;
                    }
                    DriverShellCall::Handle { wave, tx } => {
                        println!("Handle wave! {}", wave.core().method.to_string());
                        let port = wave.to().clone().unwrap_single();
                        let logger = self.skel.logger.point(port.clone().to_point()).span();
                        let router = Arc::new(self.router.clone());
                        let transmitter =
                            ProtoTransmitter::new(router, self.skel.exchanger.clone());
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
                    DriverShellCall::Ex { point, tx } => {
                        match self
                            .state
                            .get_state(point.clone().to_port().with_layer(Layer::Core))
                            .await
                        {
                            Ok(state) => {
                                tx.send(Ok(self.driver.ex(&point, state)));
                            }
                            Err(err) => {
                                tx.send(Err(err));
                            }
                        }
                    }
                    DriverShellCall::Assign { assign, rtn } => {
                        let port = assign
                            .details
                            .stub
                            .point
                            .clone()
                            .to_port()
                            .with_layer(Layer::Core);
                        let mut wave = DirectedProto::new();
                        wave.kind(DirectedKind::Ping);
                        wave.from(self.point.clone().to_port().with_layer(Layer::Shell));
                        wave.method(SysMethod::Assign);
                        let assign_ref = &assign;
                        wave.body(Substance::Sys(Sys::Assign(assign.clone())));
                        let to = self.point.clone().to_port().with_layer(Layer::Core);
                        wave.to(to.clone());
                        let wave = wave.build().unwrap();
                        let router = LayerInjectionRouter::new(
                            self.skel.clone(),
                            self.point.clone().to_port().with_layer(Layer::Driver),
                        );
                        let transmitter =
                            ProtoTransmitter::new(Arc::new(router), self.skel.exchanger.clone());
                        let ctx = RootInCtx::new(wave, to, self.logger.span(), transmitter);
                        match ctx.push() {
                            Ok(ctx) => {
                                let ctx: InCtx<'_, Sys> = ctx;
                                let ctx = ctx.push_input_ref(assign_ref);
                                let state = self.driver.assign(ctx).await;
                                match state {
                                    Ok(state) => match state {
                                        None => {
                                            rtn.send(Ok(()));
                                        }
                                        Some(state) => {
                                            rtn.send(
                                                self.skel.state.api().put_state(port, state).await,
                                            );
                                        }
                                    },
                                    Err(err) => {
                                        rtn.send(Err(err));
                                    }
                                }
                            }
                            Err(err) => {
                                rtn.send(Err(err));
                            }
                        }
                    }
                }
            }
        });
    }

    async fn traverse(&self, traversal: Traversal<UltraWave>) -> Result<(), MsgErr> {
        let core = self.core(&traversal.to.point).await?;
        if traversal.is_directed() {
            core.deliver_directed(traversal.unwrap_directed()).await;
        } else {
            core.deliver_reflected(traversal.unwrap_reflected()).await;
        }
        Ok(())
    }

    async fn lifecycle(&mut self, call: DriverLifecycleCall) -> Result<DriverStatus, MsgErr> {
        self.driver.lifecycle(call).await
    }

    async fn core(&self, point: &Point) -> Result<OuterCore<P>, MsgErr> {
        let port = point.clone().to_port().with_layer(Layer::Core);
        let (tx, mut rx) = oneshot::channel();
        self.skel
            .state
            .states_tx()
            .send(StateCall::Get {
                port: port.clone(),
                tx,
            })
            .await;
        let state = rx.await??;
        Ok(OuterCore {
            port: port.clone(),
            skel: self.skel.clone(),
            state: state.clone(),
            ex: self.driver.ex(point, state),
            router: Arc::new(self.router.clone().with(port)),
        })
    }

    #[route("Sys<Assign>")]
    async fn assign(&self, ctx: InCtx<'_, Sys>) -> Result<ReflectedCore, MsgErr> {
        match ctx.input {
            Sys::Assign(assign) => {
                let ctx = ctx.push_input_ref(assign);
                let state = self.driver.assign(ctx).await?;

                if let Some(state) = state {
                    let port = self.point.clone().to_port().with_layer(Layer::Core);
                    self.skel.state.api().put_state(port, state).await;
                }

                Ok(ReflectedCore::ok_body(Substance::Empty))
            }
            _ => Err(MsgErr::bad_request()),
        }
    }

    fn status(&self) -> &DriverStatus {
        &self.status
    }
}
