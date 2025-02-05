use crate::hyperspace::driver::{
    Driver, DriverAvail, DriverCtx, DriverSkel, HyperDriverFactory, ItemHandler, ItemSphere,
    DRIVER_BIND,
};
use crate::hyperspace::hyperspace::platform::Platform;
use crate::hyperspace::hyperspace::star::HyperStarSkel;
use once_cell::sync::Lazy;
use crate::space::artifact::ArtRef;
use crate::space::config::bind::BindConfig;
use crate::space::kind::{BaseKind, Kind};
use crate::space::parse::bind_config;
use crate::space::point::Point;
use crate::space::selector::KindSelector;
use crate::space::util::log;
use crate::space::wave::core::CoreBounce;
use crate::space::wave::exchange::asynch::{DirectedHandler, RootInCtx};
use std::str::FromStr;
use std::sync::Arc;

static BASE_BIND_CONFIG: Lazy<ArtRef<BindConfig>> = Lazy::new(|| {
    ArtRef::new(
        Arc::new(base_bind()),
        Point::from_str("GLOBAL::repo:1.0.0:/bind/base.bind").unwrap(),
    )
});

fn base_bind() -> BindConfig {
    log(bind_config(
        r#"
    Bind(version=1.0.0)
    {
    }
    "#,
    ))
    .unwrap()
}

pub struct CliDriverFactory {
    pub kind: BaseKind,
    pub avail: DriverAvail,
}

impl CliDriverFactory {
    pub fn new(avail: DriverAvail, kind: BaseKind) -> Self {
        Self { avail, kind }
    }
}

#[async_trait]
impl<P> HyperDriverFactory<P> for CliDriverFactory
where
    P: Platform,
{
    fn kind(&self) -> KindSelector {
        KindSelector::from_base(BaseKind::Cli)
    }

    async fn create(
        &self,
        skel: HyperStarSkel<P>,
        driver_skel: DriverSkel<P>,
        ctx: DriverCtx,
    ) -> Result<Box<dyn Driver<P>>, P::Err> {
        Ok(Box::new(CliDriver::new(self.avail.clone())))
    }
}

pub struct CliDriver {
    pub avail: DriverAvail,
    pub kind: BaseKind,
}

impl CliDriver {
    pub fn new(avail: DriverAvail) -> Self {
        Self { avail }
    }
}

#[async_trait]
impl<P> Driver<P> for CliDriver
where
    P: Platform,
{
    fn kind(&self) -> Kind {
        self.kind
    }

    async fn item(&self, point: &Point) -> Result<ItemSphere<P>, P::Err> {
        Ok(ItemSphere::Handler(Box::new(Cli)))
    }
}

pub struct Cli;

#[handler]
impl Cli {}

#[async_trait]
impl<P> ItemHandler<P> for Cli
where
    P: Platform,
{
    async fn bind(&self) -> Result<ArtRef<BindConfig>, P::Err> {
        Ok(DRIVER_BIND.clone())
    }
}
