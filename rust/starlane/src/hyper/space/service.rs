use crate::err::StarErr;
use crate::host::{ExeService, HostEnv, OsEnv, Proc};
use crate::hyper::space::err::HyperErr;
use crate::hyper::space::star::Templates;
use itertools::Itertools;
use nom::AsBytes;
use starlane_space::command::common::StateSrc;
use starlane_space::err::SpaceErr;
use starlane_space::hyper::{Assign, HyperSubstance};
use starlane_space::kind::Kind;
use starlane_space::loc::{Surface, ToBaseKind};
use starlane_space::log::PointLogger;
use starlane_space::particle::Status;
use starlane_space::point::Point;
use starlane_space::selector::KindSelector;
use starlane_space::settings::Timeouts;
use starlane_space::substance::Substance;
use starlane_space::util::{IdSelector, MatchSelector, OptSelector, ValueMatcher};
use starlane_space::wave::core::CoreBounce;
use starlane_space::wave::exchange::asynch::{DirectedHandler, DirectedHandlerShell, Exchanger, InCtx, RootInCtx, Router};
use starlane_space::wave::exchange::asynch::ProtoTransmitterBuilder;
use starlane_space::wave::exchange::synch::ExchangeRouter;
use starlane_space::wave::exchange::SetStrategy;
use starlane_space::wave::{Bounce, DirectedWave, ReflectedWave};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::hash::Hash;
use std::io::Read;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::process::Stdio;
use std::str::FromStr;
use std::sync::Arc;
use strum_macros::{EnumIter, EnumString};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use tokio::sync::{watch, RwLock};
use tracing::instrument::WithSubscriber;
use starlane_space::asynch::state_relay;
use starlane_space::path::Path;

pub struct ServiceCreationSelector {
    pub selector: ServiceSelector,
    pub ctx: ServiceCtx
}




#[derive(Debug,Clone,Hash,Eq,PartialEq)]
pub struct ServiceKey {
    pub name: String,
    pub kind: Kind,
    pub share: ServiceShare,
}




pub struct ServiceSelector {
    pub name:  IdSelector<String>,
    pub kind: MatchSelector<KindSelector,Kind>,
    pub share: IdSelector<ServiceShare>,
    pub star: OptSelector<IdSelector<Point>>,
    pub driver: OptSelector<IdSelector<Point>>,
    pub particle: OptSelector<IdSelector<Point>>
}


impl PartialEq<ServiceKey> for ServiceSelector {
    fn eq(&self, key: &ServiceKey) -> bool {
        self.name == key.name &&
        self.kind == key.kind &&
        self.share == key.share
    }

}
impl PartialEq<ServiceTemplate> for ServiceSelector {
    fn eq(&self, key: &ServiceTemplate) -> bool {
        self.name == key.name &&
            self.kind == key.kind
    }
}

/*
pub struct ServicePool {
    core: RwLock<ServicePoolCore>
}

impl ServicePool {


    async fn create( & self, template: &ServiceTemplate, pwd: PathBuf, mount: Point ) -> Result<ServiceStub,StarErr> {
        let mut info = template.exec.clone();
        info.stub.env.pwd = self.ctx.data_dir.join(mount.to_path()).to_str().unwrap().to_string();
        let host = info.create_host()?;
        let handler = template.dialect.handler(host)?;

        Ok(Arc::new(ServiceHandler::new(handler)))
    }
}



pub struct ServicePoolCore
{
    ctx: ServiceCtx,
    templates: Templates<ServiceTemplate>,
    services: HashMap<ServiceKey,ServiceStub>,
}

impl ServicePoolCore {

    pub fn create(&mut self, create: &ServiceCreationSelector) -> Result<Option<ServiceStub>,StarErr> {
        match self.select_from_template(&create.selector) {
            None => Ok(None),
            Some(template) => {
                let core = >ServiceCore::create( create.ctx.clone(), template )?;
                Ok(Some(ServiceRunner::new(core)))
            }
        }

    }

    pub fn select_from_template(&mut self, selector: &ServiceSelector ) -> Option<ServiceTemplate> {
        self.templates.select_one(selector).cloned()
    }
}

 */


pub trait Service where Self::Handler: DirectedHandler {
    type Handler;

    fn handler(&self) -> & Self::Handler;
}

pub struct ServiceHandler<D> where  D: DirectedHandler  {
    handler: D
}

impl <D> ServiceHandler<D> where D: DirectedHandler {

    pub fn new(handler: D) -> Self {
        Self { handler }
    }
}

impl <D> Service for ServiceHandler<D> where D: DirectedHandler{
    type Handler = D;

    fn handler(&self) -> & Self::Handler {
        & self.handler
    }
}





#[derive(Clone)]
pub enum Dialect {
    FileStore,
}

impl Dialect {
    pub fn handler(&self, host: Host) -> Result<Box<dyn DirectedHandler>, StarErr> {
        match self {
            Dialect::FileStore => {
                let cli = host.executor().ok_or("Driver ")?;
                Ok(Box::new(FileStoreCliExecutor::new(cli)))
            }
        }
    }
}


#[derive(Debug,Clone,Hash,Eq,PartialEq,EnumIter)]
pub enum ServiceShare {
    Singleton, /// one service for everyone
    Star,  /// one of this Service per star
    Driver, /// unique service per driver
    Particle, // unique service per particle
}

#[derive(Debug,Clone,Hash,Eq,PartialEq)]
pub enum ServiceAgent {
    Singleton,
    Star(Point),
    Driver {star: Point, driver: Point },
    Particle{star:Point,driver:Point,particle:Point}
}



#[derive(Debug,Clone,Eq,PartialEq,EnumString)]
pub enum ServiceShareSelector {
    Any,
    Set(HashSet<ServiceShare>)
}

impl PartialEq<ServiceShare> for ServiceShareSelector {
    fn eq(&self, other: &ServiceShare) -> bool {
        match &self {
            ServiceShareSelector::Any => true,
            ServiceShareSelector::Set(set) => set.contains(other)
        }
    }
}

impl ServiceShareSelector {
    pub fn new() -> Self {
        Self::Any
    }

    pub fn or( self, share: ServiceShare) -> Self {
        match self {
            ServiceShareSelector::Any => {
                Self::Set(HashSet::from([share]))
            }
            ServiceShareSelector::Set(mut set) => {
                set.insert(share);
                ServiceShareSelector::Set(set)
            }
        }
    }
}

impl Default for ServiceShareSelector {
    fn default() -> Self {
        Self::Any
    }
}


#[derive(Clone)]
pub struct ServiceTemplate {
    pub name: String,
    pub kind: Kind,
    pub share: ServiceShare,
    pub exec: ExeInfo<String, HostEnv, Option<Vec<String>>>,
    pub host: HostApi,
    pub dialect: Dialect,
}

impl ServiceTemplate {

     /*
        pub fn create(&self, ctx: ServiceCtx, mount: &Point) -> Result<Arc<dyn Service<Handler=Box<dyn DirectedHandler>>>, StarErr> {
            let mut exec = self.exec.clone();
            exec.stub.env.pwd = ctx.data_dir.join(mount.to_path()).to_str().unwrap().to_string();
            let host = self.exec.host.create(exec.stub.clone())?;
            let handler = self.dialect.handler(host)?;

            Ok(Arc::new(ServiceHandler::new(handler)))
        }

         */


}

impl Into<ServiceKey> for ServiceTemplate {
    fn into(self) -> ServiceKey {
        ServiceKey {
            name: self.name.clone(),
            kind: self.kind.clone(),
            share: self.share.clone(),
        }
    }
}

#[derive(Clone)]
pub struct ServiceCtx {
    pub surface: Surface,
    pub data_dir: PathBuf,
    pub router: Arc<dyn ExchangeRouter>,
    pub logger: PointLogger
}

impl ServiceCtx where {
    pub fn new(surface: Surface, data_dir: PathBuf, router: Arc<dyn ExchangeRouter>, logger: PointLogger ) -> Self {
        Self {
            surface,
            data_dir,
            router,
            logger,
        }
    }
}







#[async_trait]
pub trait Executor
where
    Self::Err: HyperErr,
{
    type Args;
    type Err;
    type Spawn;
    async fn execute(&self, args: Self::Args) -> Self::Spawn;
}

impl FileStoreCliExecutor {
    async fn assign<'a>(
        &self,
        ctx: &'a InCtx<'_, Assign>,
    ) -> Result<(), <FileStoreCliExecutor as Executor>::Err> {
        async fn wait(mut child: OsProcess, line: String) -> Result<(), StarErr> {
            match child.wait().await?.success() {
                true => Ok(()),
                false => match child.stderr.as_mut() {
                    None => Err(SpaceErr::from(format!(
                        "host operation {} failed.  No error output encountered",
                        line
                    ))
                    .into()),
                    Some(err) => {
                        let mut message = String::new();
                        err.read_to_string(&mut message).await?;
                        Err(SpaceErr::from(format!(
                            "host operation {} failed.  StdErr: {}",
                            line, message
                        ))
                        .into())
                    }
                },
            }
        }

        let bin = match &ctx.state {
            StateSrc::Substance(data) => data.to_bin()?,
            StateSrc::None => Box::new(Substance::Empty).to_bin()?,
        };
        let line = format!("write {}", ctx.details.stub.point.to_path().display());
        let args = line
            .split_whitespace()
            .map(|a| a.to_string())
            .collect::<Vec<String>>();
        let mut child = self.cli.execute(args).await?;
        let mut stdin = child.stdin.take().ok_or(SpaceErr::from(format!(
            "command {} could not write to StdIn",
            line
        )))?;
        stdin.write_all(bin.as_bytes()).await?;
        wait(child, line).await
    }
}

#[handler]
impl FileStoreCliExecutor {
    #[route("Hyp<Assign>")]
    async fn handle_assign(&self, ctx: InCtx<'_, HyperSubstance>) -> Result<(), StarErr> {
        if let HyperSubstance::Assign(assign) = ctx.input {
            let ctx = ctx.push_input_ref(assign);
            ctx.logger.result(self.assign(&ctx).await)
        } else {
            Err(StarErr::new("Bad Reqeust: expected Assign"))
        }
    }
}

pub struct OsProcess {
    child: Child,
}

impl Deref for OsProcess {
    type Target = Child;

    fn deref(&self) -> &Self::Target {
        &self.child
    }
}

impl DerefMut for OsProcess {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.child
    }
}

impl OsProcess {
    pub fn new(child: Child) -> Self {
        Self { child }
    }
}

impl Proc for OsProcess {
    type StdOut = ChildStdout;
    type StdIn = ChildStdin;
    type StdErr = ChildStderr;

    fn stderr(&self) -> Option<&Self::StdErr> {
        self.child.stderr.as_ref()
    }

    fn stdout(&self) -> Option<&Self::StdOut> {
        self.child.stdout.as_ref()
    }

    fn stdin(&mut self) -> Option<&Self::StdIn> {
        self.child.stdin.as_ref()
    }
}

#[async_trait]
impl Executor for OsExeCli {
    type Args = Vec<String>;
    type Err = StarErr;
    type Spawn = Result<OsProcess, Self::Err>;

    async fn execute(&self, args: Self::Args) -> Self::Spawn {
        let mut command = Command::new(self.stub.loc.clone());
        command.envs(self.stub.env.env.clone());
        command.args(args);
        command.current_dir(self.stub.env.pwd.clone());
        command.env_clear();
        command.stdin(Stdio::piped()).output().await?;
        command.stdout(Stdio::piped()).output().await?;
        command.stderr(Stdio::piped()).output().await?;

        let child = command.spawn()?;
        Ok(OsProcess::new(child))
    }
}



#[derive(Clone)]
pub struct OsExeCli {
    pub stub: OsExeStub,
}

impl OsExeCli {
    pub fn new<I>(info: I) -> Self
    where
        I: Into<OsExeStub>,
    {
        let info = info.into();
        Self { stub: info }
    }
}


#[derive(DirectedHandler)]
pub struct FileStoreCliExecutor {
    pub cli: Box<dyn Executor<Args = Vec<String>, Spawn = Result<OsProcess, StarErr>, Err = StarErr>+Send+Sync>
}

impl FileStoreCliExecutor {
    pub fn new(cli: Box<dyn Executor<Args = Vec<String>, Spawn = Result<OsProcess, StarErr>, Err = StarErr>+Send+Sync >) -> Self {
        Self { cli }
    }
}

#[async_trait]
impl Executor for FileStoreCliExecutor {
    type Args = RootInCtx;
    type Err = StarErr;
    type Spawn = CoreBounce;

    async fn execute(&self, args: Self::Args) -> Self::Spawn {
        DirectedHandler::handle(self, args).await
    }
}



#[derive(Clone, Hash, Eq, PartialEq)]
pub enum HostApi {
    Cli(HostKind),
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub enum HostKind {
    Os,
}

pub enum Host {
    Cli(CliHost),
}

impl Host {
    pub fn is_cli(&self) -> bool {
        match self {
            Host::Cli(_) => true,
        }
    }

    pub fn executor(
        &self,
    ) -> Option<Box<dyn Executor<Spawn = Result<OsProcess,StarErr>, Err = StarErr, Args = Vec<String>>+Send+Sync>> {
        match self {
            Host::Cli(CliHost::Os(exec)) => Some(Box::new(exec.clone())),
        }
    }
}


pub enum CliHost {
    Os(OsExeCli),
}

impl CliHost {
    pub fn executor(&self) -> &OsExeCli {
        match self {
            CliHost::Os(exec) => exec,
        }
    }
}

impl Host {}

impl HostApi {
    pub fn create<S>(&self, stub: S) -> Result<Host, StarErr>
    where
        S: Into<OsExeStub>,
    {
        match self {
            HostApi::Cli(HostKind::Os) => {
                let exe = OsExeCli::new(stub);
                let host = CliHost::Os(exe);
                let host = Host::Cli(host);
                Ok(host)
            }
        }
    }
}

impl Into<OsExeStub> for ExeStub<String, HostEnv, Option<Vec<String>>> {
    fn into(self) -> OsExeStub {
        OsExeStub::new( self.loc.into(), self.env.into(), () )
    }
}
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct ExeStub<L, E, A>
where
    E: Clone + Hash + Eq + PartialEq,
    L: Clone + Hash + Eq + PartialEq,
    A: Clone + Hash + Eq + PartialEq,
{
    pub loc: L,
    pub env: E,
    pub args: A,
}

impl<L, E, A> ExeStub<L, E, A>
where
    E: Clone + Hash + Eq + PartialEq,
    L: Clone + Hash + Eq + PartialEq,
    A: Clone + Hash + Eq + PartialEq,
{
    pub fn new(loc: L, env: E, args: A) -> Self {
        Self { loc, env, args }
    }
}

impl<E> Into<ExeStub<PathBuf, OsEnv, ()>> for ExeStub<String, E, ()>
where
    E: Into<HostEnv> + Clone + Hash + Eq + PartialEq,
{
    fn into(self) -> ExeStub<PathBuf, HostEnv, ()> {
        ExeStub {
            loc: self.loc.into(),
            env: self.env.into(),
            args: (),
        }
    }
}

pub type OsExeInfo = ExeInfo<PathBuf, OsEnv, ()>;
pub type OsExeStub = ExeStub<PathBuf, OsEnv, ()>;
pub type OsExeStubArgs = ExeStub<PathBuf, HostEnv, Vec<String>>;

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct ExeInfo<L, E, A>
where
    E: Clone + Hash + Eq + PartialEq,
    L: Clone + Hash + Eq + PartialEq,
    A: Clone + Hash + Eq + PartialEq,
{
    pub host: HostApi,
    pub stub: ExeStub<L, E, A>,
}

impl<L, E, A> ExeInfo<L, E, A>
where
    E: Clone + Hash + Eq + PartialEq,
    L: Clone + Hash + Eq + PartialEq,
    A: Clone + Hash + Eq + PartialEq,
{
    pub fn new(host: HostApi, stub: ExeStub<L, E, A>) -> Self {
        Self { host, stub }
    }


}

impl<L, E, A> ExeInfo<L, E, A>
where
    L: Clone + Hash + Eq + PartialEq+Into<PathBuf>,
    E: Clone + Hash + Eq + PartialEq+Into<HostEnv>,
    A: Clone + Hash + Eq + PartialEq,

{
    pub fn create_host(self) -> Result<Host,StarErr> {
        self.host.create(&self.stub)
    }
}

impl <L,E,A> From<&ExeStub<L, E, A>> for ExeStub<PathBuf, HostEnv, ()> where L:  Clone+Hash+Eq+PartialEq+Into<PathBuf>, E: Clone+Hash+Eq+PartialEq+Into<HostEnv>, A: Clone+Hash+Eq+PartialEq  {
    fn from(stub: &ExeStub<L, E, A>) -> Self {
        let path = stub.loc.clone().into();
        let env = stub.env.clone().into();

        ExeStub::new(path,env,())
    }
}


pub struct ServiceCall {
    pub from: Point,
    pub tx: tokio::sync::oneshot::Sender<Bounce<ReflectedWave>>,
    pub command: ServiceCommand
}

pub enum ServiceCommand {
    DirectedWave(DirectedWave)
}



#[derive(Clone)]
pub struct ServiceStub {
    template: ServiceTemplate,
    call_tx: tokio::sync::mpsc::Sender<ServiceCall>,
    status_rx: watch::Receiver<Status>,
}

pub struct ServiceRunner<D> where D: DirectedHandler + 'static {
    call_rx: tokio::sync::mpsc::Receiver<ServiceCall>,
    status_tx: tokio::sync::mpsc::Sender<Status>,
    core: ServiceCore<D>,
}

impl <D> ServiceRunner <D> where D: DirectedHandler {
    fn new( core: ServiceCore<D> )  -> ServiceStub {
        let (call_tx, call_rx) = tokio::sync::mpsc::channel(1024);
        let( status_tx, status_rx) = state_relay(Status::Pending);
        let template = core.template.clone();
        let rtn = ServiceStub {
            call_tx,
            status_rx,
            template,
        };

        let runner = Self{ call_rx,  status_tx, core };

        tokio::spawn( async move {
            runner.launch().await
        });

        rtn
    }

    async fn launch(mut self)  {
        let status_tx = self.status_tx.clone();
        let logger = self.core.ctx.logger.clone();
        match logger.result(self.run().await) {
            Ok(status) => {
                status_tx.send(status);
            }
            Err(_) => {
                status_tx.send(Status::Panic);
            }
        }
    }

    async fn run(mut self) -> Result<Status,StarErr> {

        self.status_tx.send(Status::Ready);

        while let Some(call) = self.call_rx.recv().await {
            match call.command {
                ServiceCommand::DirectedWave(wave) => {
                    self.core.handler.handle( wave ).await;
                }
            }
        }

        Ok(Status::Done)
    }
}

struct ServiceCore<D> where D: DirectedHandler {
    ctx: ServiceCtx,
    template: ServiceTemplate,
    handler: DirectedHandlerShell<D>
}

impl <D> ServiceCore<D> where D: DirectedHandler {
    /*
    pub fn create(ctx: ServiceCtx, template: ServiceTemplate ) -> Result<Self,StarErr>{
        let host = template.host.create( template.exec.stub.clone() )?;
        let exchanger= Exchanger::new(ctx.surface.clone(), Timeouts::default(), ctx.logger.clone() );
        let mut builder = ProtoTransmitterBuilder::new(ctx.router.clone(), exchanger);
        builder.from = SetStrategy::Override(ctx.surface.clone());
        let handler = template.dialect.handler(host)?;
        let handler = DirectedHandlerShell::new( handler, builder, ctx.surface.clone(), ctx.logger.logger.clone());
        Ok(Self {
            ctx,
            template,
            handler
        })
    }

     */

    /*
    pub fn handler( & self ) -> D {
        self.handler.clone()
    }

     */
}




#[cfg(test)]
pub mod tests {
    use crate::host::{HostEnv, OsEnv};
    use crate::hyper::space::service::{ExeInfo, HostApi};
    use crate::hyper::space::service::{ExeStub, HostKind};
    use std::env;
    use std::path::absolute;
    use std::process::Stdio;
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
    use tokio_util::codec::{FramedRead, LinesCodec};

    #[tokio::test]
    pub async fn test_os_cli_host() {
        let mut builder = HostEnv::builder();
        builder.pwd(
            absolute(env::current_dir().unwrap())
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
        );
        builder.env("DATA_DIR", "./");
        let env = builder.build();
        let path = "./cli-host.sh".to_string();
        let args: Option<Vec<String>> = Option::None;
        let stub: ExeStub<String, OsEnv, Option<Vec<String>>> = ExeStub::new(path, env, None);
        let info = ExeInfo::new(HostApi::Cli(HostKind::Os), stub);

        let host = info.create_host().unwrap();
        let executor = host.executor().unwrap();
        let mut child = executor.execute(vec![]).await.unwrap();


        let stdout = child.stdout.take()
            .expect("child did not have a handle to stdout");

        let mut reader = BufReader::new(stdout).lines();

        /*
        // Ensure the child process is spawned in the runtime so it can
        // make progress on its own while we await for any output.
        tokio::spawn(async move {
            let status = child.wait().await
                .expect("child process encountered an error");

            println!("child status was: {}", status);
        });

         */

        while let Some(line) = reader.next_line().await.unwrap() {
            println!("Line: {}", line);
        }


    }
}