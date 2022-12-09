use core::str::FromStr;
use cosmic_nom::{new_span, Trace};
use nom::combinator::all_consuming;
use serde::{Deserialize, Serialize};

use crate::err::{ParseErrs, SpaceErr};
use crate::loc::{
    PointSegQuery, PointSegment, RouteSegQuery, Surface, ToPoint, ToSurface, Variable, Version,
    CENTRAL, GLOBAL_EXEC, GLOBAL_LOGGER, GLOBAL_REGISTRY, LOCAL_ENDPOINT, LOCAL_HYPERGATE,
    LOCAL_PORTAL, LOCAL_STAR, REMOTE_ENDPOINT,
};
use crate::parse::error::result;
use crate::parse::{
    consume_point, consume_point_ctx, point_route_segment, point_selector, point_var, Env,
    ResolverErr,
};
use crate::selector::Selector;
use crate::util::ToResolved;
use crate::wave::{Agent, Recipients, ToRecipients};
use crate::{ANONYMOUS, HYPERUSER, HYPER_USERBASE};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum RouteSeg {
    This,
    Local,
    Remote,
    Global,
    Domain(String),
    Tag(String),
    Star(String),
    Hyper,
}

impl RouteSegQuery for RouteSeg {
    fn is_local(&self) -> bool {
        match self {
            RouteSeg::This => true,
            _ => false,
        }
    }

    fn is_global(&self) -> bool {
        match self {
            RouteSeg::Global => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum RouteSegVar {
    This,
    Local,
    Remote,
    Global,
    Hyper,
    Domain(String),
    Tag(String),
    Star(String),
    Var(Variable),
}

impl RouteSegQuery for RouteSegVar {
    fn is_local(&self) -> bool {
        match self {
            RouteSegVar::This => true,
            _ => false,
        }
    }

    fn is_global(&self) -> bool {
        match self {
            RouteSegVar::Global => true,
            _ => false,
        }
    }
}

impl TryInto<RouteSeg> for RouteSegVar {
    type Error = SpaceErr;

    fn try_into(self) -> Result<RouteSeg, Self::Error> {
        match self {
            RouteSegVar::This => Ok(RouteSeg::This),
            RouteSegVar::Local => Ok(RouteSeg::Local),
            RouteSegVar::Global => Ok(RouteSeg::Global),
            RouteSegVar::Domain(domain) => Ok(RouteSeg::Domain(domain)),
            RouteSegVar::Tag(tag) => Ok(RouteSeg::Tag(tag)),
            RouteSegVar::Star(star) => Ok(RouteSeg::Star(star)),
            RouteSegVar::Var(var) => Err(ParseErrs::from_range(
                "variables not allowed in this context",
                "variable not allowed here",
                var.trace.range,
                var.trace.extra,
            )),
            RouteSegVar::Remote => Ok(RouteSeg::Remote),
            RouteSegVar::Hyper => Ok(RouteSeg::Hyper),
        }
    }
}

impl Into<RouteSegVar> for RouteSeg {
    fn into(self) -> RouteSegVar {
        match self {
            RouteSeg::This => RouteSegVar::This,
            RouteSeg::Local => RouteSegVar::Local,
            RouteSeg::Remote => RouteSegVar::Remote,
            RouteSeg::Hyper => RouteSegVar::Hyper,
            RouteSeg::Global => RouteSegVar::Global,
            RouteSeg::Domain(domain) => RouteSegVar::Domain(domain),
            RouteSeg::Tag(tag) => RouteSegVar::Tag(tag),
            RouteSeg::Star(mesh) => RouteSegVar::Star(mesh),
        }
    }
}

impl ToString for RouteSegVar {
    fn to_string(&self) -> String {
        match self {
            Self::This => ".".to_string(),
            Self::Local => "LOCAL".to_string(),
            Self::Remote => "REMOTE".to_string(),
            Self::Global => "GLOBAL".to_string(),
            Self::Domain(domain) => domain.clone(),
            Self::Tag(tag) => {
                format!("[{}]", tag)
            }
            Self::Star(mesh) => {
                format!("<<{}>>", mesh)
            }
            Self::Var(var) => {
                format!("${{{}}}", var.name)
            }
            RouteSegVar::Hyper => "HYPER".to_string(),
        }
    }
}

impl FromStr for RouteSeg {
    type Err = SpaceErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = new_span(s);
        Ok(all_consuming(point_route_segment)(s)?.1)
    }
}

impl ToString for RouteSeg {
    fn to_string(&self) -> String {
        match self {
            RouteSeg::This => ".".to_string(),
            RouteSeg::Domain(domain) => domain.clone(),
            RouteSeg::Tag(tag) => {
                format!("[{}]", tag)
            }
            RouteSeg::Star(sys) => {
                format!("<<{}>>", sys)
            }
            RouteSeg::Global => "GLOBAL".to_string(),
            RouteSeg::Local => "LOCAL".to_string(),
            RouteSeg::Remote => "REMOTE".to_string(),
            RouteSeg::Hyper => "HYPER".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, strum_macros::Display)]
pub enum PointSegKind {
    Root,
    Space,
    Base,
    FilesystemRootDir,
    Dir,
    File,
    Version,
    Pop,
    Working,
    Var,
}

impl PointSegKind {
    pub fn preceding_delim(&self, post_fileroot: bool) -> &'static str {
        match self {
            Self::Space => "",
            Self::Base => ":",
            Self::Dir => "",
            Self::File => "",
            Self::Version => ":",
            Self::FilesystemRootDir => ":",
            Self::Root => "",
            Self::Pop => match post_fileroot {
                true => "",
                false => ":",
            },
            Self::Working => match post_fileroot {
                true => "",
                false => ":",
            },
            Self::Var => match post_fileroot {
                true => "",
                false => ":",
            },
        }
    }

    pub fn is_normalized(&self) -> bool {
        match self {
            Self::Pop => false,
            Self::Working => false,
            Self::Var => false,
            _ => true,
        }
    }

    pub fn is_version(&self) -> bool {
        match self {
            Self::Version => true,
            _ => false,
        }
    }

    pub fn is_file(&self) -> bool {
        match self {
            Self::File => true,
            _ => false,
        }
    }

    pub fn is_dir(&self) -> bool {
        match self {
            Self::Dir => true,
            _ => false,
        }
    }

    pub fn is_filesystem_seg(&self) -> bool {
        match self {
            PointSegKind::Root => false,
            PointSegKind::Space => false,
            PointSegKind::Base => false,
            PointSegKind::FilesystemRootDir => true,
            PointSegKind::Dir => true,
            PointSegKind::File => true,
            PointSegKind::Version => false,
            PointSegKind::Pop => true,
            PointSegKind::Working => true,
            PointSegKind::Var => true,
        }
    }

    pub fn is_mesh_seg(&self) -> bool {
        match self {
            PointSegKind::Root => true,
            PointSegKind::Space => true,
            PointSegKind::Base => true,
            PointSegKind::FilesystemRootDir => false,
            PointSegKind::Dir => false,
            PointSegKind::File => false,
            PointSegKind::Version => true,
            PointSegKind::Pop => true,
            PointSegKind::Working => true,
            PointSegKind::Var => true,
        }
    }
}

impl PointSegQuery for PointSeg {
    fn is_filesystem_root(&self) -> bool {
        match self {
            Self::FilesystemRootDir => true,
            _ => false,
        }
    }
    fn kind(&self) -> PointSegKind {
        match self {
            PointSeg::Root => PointSegKind::Root,
            PointSeg::Space(_) => PointSegKind::Space,
            PointSeg::Base(_) => PointSegKind::Base,
            PointSeg::FilesystemRootDir => PointSegKind::FilesystemRootDir,
            PointSeg::Dir(_) => PointSegKind::Dir,
            PointSeg::File(_) => PointSegKind::File,
            PointSeg::Version(_) => PointSegKind::Version,
        }
    }
}

impl PointSegQuery for PointSegCtx {
    fn is_filesystem_root(&self) -> bool {
        match self {
            Self::FilesystemRootDir => true,
            _ => false,
        }
    }

    fn kind(&self) -> PointSegKind {
        match self {
            Self::Root => PointSegKind::Root,
            Self::Space(_) => PointSegKind::Space,
            Self::Base(_) => PointSegKind::Base,
            Self::FilesystemRootDir => PointSegKind::FilesystemRootDir,
            Self::Dir(_) => PointSegKind::Dir,
            Self::File(_) => PointSegKind::File,
            Self::Version(_) => PointSegKind::Version,
            Self::Pop { .. } => PointSegKind::Pop,
            Self::Working { .. } => PointSegKind::Working,
        }
    }
}

impl PointSegQuery for PointSegVar {
    fn is_filesystem_root(&self) -> bool {
        match self {
            Self::FilesystemRootDir => true,
            _ => false,
        }
    }

    fn kind(&self) -> PointSegKind {
        match self {
            Self::Root => PointSegKind::Root,
            Self::Space(_) => PointSegKind::Space,
            Self::Base(_) => PointSegKind::Base,
            Self::FilesystemRootDir => PointSegKind::FilesystemRootDir,
            Self::Dir(_) => PointSegKind::Dir,
            Self::File(_) => PointSegKind::File,
            Self::Version(_) => PointSegKind::Version,
            Self::Pop { .. } => PointSegKind::Pop,
            Self::Working { .. } => PointSegKind::Working,
            Self::Var(_) => PointSegKind::Var,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum PointSegCtx {
    Root,
    Space(String),
    Base(String),
    FilesystemRootDir,
    Dir(String),
    File(String),
    Version(Version),
    Working(Trace),
    Pop(Trace),
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum PointSegVar {
    Root,
    Space(String),
    Base(String),
    FilesystemRootDir,
    Dir(String),
    File(String),
    Version(Version),
    Working(Trace),
    Pop(Trace),
    Var(Variable),
}

impl ToString for PointSegVar {
    fn to_string(&self) -> String {
        match self {
            PointSegVar::Root => "".to_string(),
            PointSegVar::Space(space) => space.clone(),
            PointSegVar::Base(base) => base.clone(),
            PointSegVar::FilesystemRootDir => "/".to_string(),
            PointSegVar::Dir(dir) => dir.clone(),
            PointSegVar::File(file) => file.clone(),
            PointSegVar::Version(version) => version.to_string(),
            PointSegVar::Working(_) => ".".to_string(),
            PointSegVar::Pop(_) => "..".to_string(),
            PointSegVar::Var(var) => format!("${{{}}}", var.name),
        }
    }
}

impl PointSegVar {
    pub fn is_normalized(&self) -> bool {
        self.kind().is_normalized()
    }

    pub fn is_filesystem_seg(&self) -> bool {
        self.kind().is_filesystem_seg()
    }
}

impl Into<PointSegVar> for PointSegCtx {
    fn into(self) -> PointSegVar {
        match self {
            PointSegCtx::Root => PointSegVar::Root,
            PointSegCtx::Space(space) => PointSegVar::Space(space),
            PointSegCtx::Base(base) => PointSegVar::Base(base),
            PointSegCtx::FilesystemRootDir => PointSegVar::FilesystemRootDir,
            PointSegCtx::Dir(dir) => PointSegVar::Dir(dir),
            PointSegCtx::File(file) => PointSegVar::File(file),
            PointSegCtx::Version(version) => PointSegVar::Version(version),
            PointSegCtx::Working(trace) => PointSegVar::Working(trace),
            PointSegCtx::Pop(trace) => PointSegVar::Pop(trace),
        }
    }
}

impl TryInto<PointSegCtx> for PointSegVar {
    type Error = SpaceErr;

    fn try_into(self) -> Result<PointSegCtx, Self::Error> {
        match self {
            PointSegVar::Root => Ok(PointSegCtx::Root),
            PointSegVar::Space(space) => Ok(PointSegCtx::Space(space)),
            PointSegVar::Base(base) => Ok(PointSegCtx::Base(base)),
            PointSegVar::FilesystemRootDir => Ok(PointSegCtx::FilesystemRootDir),
            PointSegVar::Dir(dir) => Ok(PointSegCtx::Dir(dir)),
            PointSegVar::File(file) => Ok(PointSegCtx::File(file)),
            PointSegVar::Version(version) => Ok(PointSegCtx::Version(version)),
            PointSegVar::Working(trace) => Err(ParseErrs::from_range(
                "working point not available in this context",
                "working point not available",
                trace.range,
                trace.extra,
            )),
            PointSegVar::Pop(trace) => Err(ParseErrs::from_range(
                "point pop not available in this context",
                "point pop not available",
                trace.range,
                trace.extra,
            )),
            PointSegVar::Var(var) => Err(ParseErrs::from_range(
                "variable substitution not available in this context",
                "var subst not available",
                var.trace.range,
                var.trace.extra,
            )),
        }
    }
}

impl TryInto<PointSeg> for PointSegCtx {
    type Error = SpaceErr;

    fn try_into(self) -> Result<PointSeg, Self::Error> {
        match self {
            PointSegCtx::Root => Ok(PointSeg::Root),
            PointSegCtx::Space(space) => Ok(PointSeg::Space(space)),
            PointSegCtx::Base(base) => Ok(PointSeg::Base(base)),
            PointSegCtx::FilesystemRootDir => Ok(PointSeg::FilesystemRootDir),
            PointSegCtx::Dir(dir) => Ok(PointSeg::Dir(dir)),
            PointSegCtx::File(file) => Ok(PointSeg::File(file)),
            PointSegCtx::Version(version) => Ok(PointSeg::Version(version)),
            PointSegCtx::Working(trace) => Err(ParseErrs::from_range(
                "working point not available in this context",
                "working point not available",
                trace.range,
                trace.extra,
            )),
            PointSegCtx::Pop(trace) => Err(ParseErrs::from_range(
                "point pop not available in this context",
                "point pop not available",
                trace.range,
                trace.extra,
            )),
        }
    }
}

impl PointSegCtx {
    pub fn is_normalized(&self) -> bool {
        self.kind().is_normalized()
    }

    pub fn is_filesystem_seg(&self) -> bool {
        self.kind().is_filesystem_seg()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum PointSeg {
    Root,
    Space(String),
    Base(String),
    FilesystemRootDir,
    Dir(String),
    File(String),
    Version(Version),
}

impl PointSegment for PointSeg {}

impl PointSegment for PointSegCtx {}

impl PointSegment for PointSegVar {}

impl Into<PointSegCtx> for PointSeg {
    fn into(self) -> PointSegCtx {
        match self {
            PointSeg::Root => PointSegCtx::Root,
            PointSeg::Space(space) => PointSegCtx::Space(space),
            PointSeg::Base(base) => PointSegCtx::Base(base),
            PointSeg::FilesystemRootDir => PointSegCtx::FilesystemRootDir,
            PointSeg::Dir(dir) => PointSegCtx::Dir(dir),
            PointSeg::File(file) => PointSegCtx::File(file),
            PointSeg::Version(version) => PointSegCtx::Version(version),
        }
    }
}

impl PointSeg {
    pub fn is_file(&self) -> bool {
        self.kind().is_file()
    }

    pub fn is_normalized(&self) -> bool {
        self.kind().is_normalized()
    }

    pub fn is_version(&self) -> bool {
        self.kind().is_version()
    }

    pub fn is_filesystem_seg(&self) -> bool {
        self.kind().is_filesystem_seg()
    }
    pub fn preceding_delim(&self, post_fileroot: bool) -> &str {
        self.kind().preceding_delim(post_fileroot)
    }
}

impl ToString for PointSeg {
    fn to_string(&self) -> String {
        match self {
            PointSeg::Space(space) => space.clone(),
            PointSeg::Base(base) => base.clone(),
            PointSeg::Dir(dir) => dir.clone(),
            PointSeg::File(file) => file.clone(),
            PointSeg::Version(version) => version.to_string(),
            PointSeg::FilesystemRootDir => "/".to_string(),
            PointSeg::Root => "".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PointSegDelim {
    Empty,
    Mesh,
    File,
}

impl ToString for PointSegDelim {
    fn to_string(&self) -> String {
        match self {
            PointSegDelim::Empty => "".to_string(),
            PointSegDelim::Mesh => ":".to_string(),
            PointSegDelim::File => "/".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointSegPairDef<Seg> {
    pub delim: PointSegDelim,
    pub seg: Seg,
}

impl<Seg> PointSegPairDef<Seg> {
    pub fn new(delim: PointSegDelim, seg: Seg) -> Self {
        Self { delim, seg }
    }
}

impl<Seg> ToString for PointSegPairDef<Seg>
where
    Seg: ToString,
{
    fn to_string(&self) -> String {
        format!("{}{}", self.delim.to_string(), self.seg.to_string())
    }
}

impl Into<Surface> for Point {
    fn into(self) -> Surface {
        Surface {
            point: self,
            topic: Default::default(),
            layer: Default::default(),
        }
    }
}

impl ToRecipients for Point {
    fn to_recipients(self) -> Recipients {
        self.to_surface().to_recipients()
    }
}

impl PointVar {
    pub fn to_point(self) -> Result<Point, SpaceErr> {
        self.collapse()
    }

    pub fn to_point_ctx(self) -> Result<PointCtx, SpaceErr> {
        self.collapse()
    }
}

impl ToPoint for Point {
    fn to_point(&self) -> Point {
        self.clone()
    }
}

impl ToSurface for Point {
    fn to_surface(&self) -> Surface {
        self.clone().into()
    }
}

impl ToResolved<Point> for PointVar {
    fn to_resolved(self, env: &Env) -> Result<Point, SpaceErr> {
        let point_ctx: PointCtx = self.to_resolved(env)?;
        point_ctx.to_resolved(env)
    }
}

impl Into<Selector> for Point {
    fn into(self) -> Selector {
        let string = self.to_string();
        let rtn = result(all_consuming(point_selector)(new_span(string.as_str()))).unwrap();
        string;
        rtn
    }
}

impl PointCtx {
    pub fn to_point(self) -> Result<Point, SpaceErr> {
        self.collapse()
    }
}

impl ToResolved<PointCtx> for PointVar {
    fn collapse(self) -> Result<PointCtx, SpaceErr> {
        let route = self.route.try_into()?;
        let mut segments = vec![];
        for segment in self.segments {
            segments.push(segment.try_into()?);
        }
        Ok(PointCtx { route, segments })
    }

    fn to_resolved(self, env: &Env) -> Result<PointCtx, SpaceErr> {
        let mut rtn = String::new();
        let mut after_fs = false;
        let mut errs = vec![];

        match &self.route {
            RouteSegVar::Var(var) => match env.val(var.name.clone().as_str()) {
                Ok(val) => {
                    let val: String = val.clone().try_into()?;
                    rtn.push_str(format!("{}::", val.as_str()).as_str());
                }
                Err(err) => match err {
                    ResolverErr::NotAvailable => {
                        errs.push(ParseErrs::from_range(
                            format!(
                                "variables not available in this context '{}'",
                                var.name.clone()
                            )
                            .as_str(),
                            "Not Available",
                            var.trace.range.clone(),
                            var.trace.extra.clone(),
                        ));
                    }
                    ResolverErr::NotFound => {
                        errs.push(ParseErrs::from_range(
                            format!("variable could not be resolved '{}'", var.name.clone())
                                .as_str(),
                            "Not Found",
                            var.trace.range.clone(),
                            var.trace.extra.clone(),
                        ));
                    }
                },
            },

            RouteSegVar::This => {}
            RouteSegVar::Domain(domain) => {
                rtn.push_str(format!("{}::", domain).as_str());
            }
            RouteSegVar::Tag(tag) => {
                rtn.push_str(format!("[{}]::", tag).as_str());
            }
            RouteSegVar::Star(mesh) => {
                rtn.push_str(format!("<{}>::", mesh).as_str());
            }
            RouteSegVar::Global => {
                rtn.push_str("GLOBAL::");
            }
            RouteSegVar::Local => {
                rtn.push_str("LOCAL::");
            }
            RouteSegVar::Remote => {
                rtn.push_str("REMOTE::");
            }
            RouteSegVar::Hyper => {
                rtn.push_str("HYPER::");
            }
        };

        if self.segments.len() == 0 {
            rtn.push_str("ROOT");
            return consume_point_ctx(rtn.as_str());
        }
        for (index, segment) in self.segments.iter().enumerate() {
            if let PointSegVar::Var(ref var) = segment {
                match env.val(var.name.clone().as_str()) {
                    Ok(val) => {
                        if index > 1 {
                            if after_fs {
                                //                                    rtn.push_str("/");
                            } else {
                                rtn.push_str(":");
                            }
                        }
                        let val: String = val.clone().try_into()?;
                        rtn.push_str(val.as_str());
                    }
                    Err(err) => match err {
                        ResolverErr::NotAvailable => {
                            errs.push(ParseErrs::from_range(
                                format!(
                                    "variables not available in this context '{}'",
                                    var.name.clone()
                                )
                                .as_str(),
                                "Not Available",
                                var.trace.range.clone(),
                                var.trace.extra.clone(),
                            ));
                        }
                        ResolverErr::NotFound => {
                            errs.push(ParseErrs::from_range(
                                format!("variable could not be resolved '{}'", var.name.clone())
                                    .as_str(),
                                "Not Found",
                                var.trace.range.clone(),
                                var.trace.extra.clone(),
                            ));
                        }
                    },
                }
            } else if PointSegVar::FilesystemRootDir == *segment {
                after_fs = true;
                rtn.push_str(":/");
            } else {
                if index > 0 {
                    if after_fs {
                        //rtn.push_str("/");
                    } else {
                        rtn.push_str(":");
                    }
                }
                rtn.push_str(segment.to_string().as_str());
            }
        }
        if self.is_dir() {
            //rtn.push_str("/");
        }

        if !errs.is_empty() {
            let errs = ParseErrs::fold(errs);
            return Err(errs.into());
        }
        consume_point_ctx(rtn.as_str())
    }
}

impl ToResolved<Point> for PointCtx {
    fn collapse(self) -> Result<Point, SpaceErr> {
        let mut segments = vec![];
        for segment in self.segments {
            segments.push(segment.try_into()?);
        }
        Ok(Point {
            route: self.route,
            segments,
        })
    }

    fn to_resolved(self, env: &Env) -> Result<Point, SpaceErr> {
        if self.segments.is_empty() {
            return Ok(Point {
                route: self.route,
                segments: vec![],
            });
        }

        let mut old = self;
        let mut point = Point::root();
        point.route = old.route.clone();

        for (index, segment) in old.segments.iter().enumerate() {
            match segment {
                PointSegCtx::Working(trace) => {
                    if index > 1 {
                        return Err(ParseErrs::from_range(
                            "working point can only be referenced in the first point segment",
                            "first segment only",
                            trace.range.clone(),
                            trace.extra.clone(),
                        ));
                    }
                    point = match env.point_or() {
                        Ok(point) => point.clone(),
                        Err(_) => {
                            return Err(ParseErrs::from_range(
                                "working point is not available in this context",
                                "not available",
                                trace.range.clone(),
                                trace.extra.clone(),
                            ));
                        }
                    };
                }
                PointSegCtx::Pop(trace) => {
                    if index <= 1 {
                        point = match env.point_or() {
                            Ok(point) => point.clone(),
                            Err(_) => {
                                return Err(ParseErrs::from_range(
                                    "cannot pop because working point is not available in this context",
                                    "not available",
                                    trace.range.clone(),
                                    trace.extra.clone(),
                                ));
                            }
                        };
                    }
                    if point.segments.pop().is_none() {
                        return Err(ParseErrs::from_range(
                            format!(
                                "Too many point pops. working point was: '{}'",
                                env.point_or().unwrap().to_string()
                            )
                            .as_str(),
                            "too many point pops",
                            trace.range.clone(),
                            trace.extra.clone(),
                        ));
                    }
                }
                PointSegCtx::FilesystemRootDir => {
                    point = point.push("/".to_string())?;
                }
                PointSegCtx::Root => {
                    //segments.push(PointSeg::Root)
                }
                PointSegCtx::Space(space) => point = point.push(space.clone())?,
                PointSegCtx::Base(base) => point = point.push(base.clone())?,
                PointSegCtx::Dir(dir) => point = point.push(dir.clone())?,
                PointSegCtx::File(file) => point = point.push(file.clone())?,
                PointSegCtx::Version(version) => point = point.push(version.to_string())?,
            }
        }

        point.route = old.route.clone();
        Ok(point)
    }
}

impl TryInto<Point> for PointCtx {
    type Error = SpaceErr;

    fn try_into(self) -> Result<Point, Self::Error> {
        let mut rtn = vec![];
        for segment in self.segments {
            rtn.push(segment.try_into()?);
        }
        Ok(Point {
            route: self.route,
            segments: rtn,
        })
    }
}

impl TryFrom<String> for Point {
    type Error = SpaceErr;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        consume_point(value.as_str())
    }
}

impl TryFrom<&str> for Point {
    type Error = SpaceErr;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        consume_point(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct PointDef<Route, Seg> {
    pub route: Route,
    pub segments: Vec<Seg>,
}

impl<Route, Seg> PointDef<Route, Seg>
where
    Route: Clone,
    Seg: Clone,
{
    pub fn parent(&self) -> Option<PointDef<Route, Seg>> {
        if self.segments.is_empty() {
            return None;
        }
        let mut segments = self.segments.clone();
        segments.remove(segments.len() - 1);
        Some(Self {
            route: self.route.clone(),
            segments,
        })
    }

    pub fn last_segment(&self) -> Option<Seg> {
        self.segments.last().cloned()
    }

    pub fn is_root(&self) -> bool {
        self.segments.is_empty()
    }
}

impl Point {
    pub fn to_agent(&self) -> Agent {
        if *self == *HYPERUSER {
            Agent::HyperUser
        } else if *self == *ANONYMOUS {
            Agent::Anonymous
        } else {
            Agent::Point(self.clone())
        }
    }

    pub fn is_global(&self) -> bool {
        match self.route {
            RouteSeg::Global => true,
            _ => false,
        }
    }

    pub fn is_parent_of(&self, point: &Point) -> bool {
        if self.segments.len() > point.segments.len() {
            return false;
        }

        if self.route != point.route {
            return false;
        }

        for i in 0..self.segments.len() {
            if *self.segments.get(i).as_ref().unwrap() != *point.segments.get(i).as_ref().unwrap() {
                return false;
            }
        }
        true
    }

    pub fn central() -> Self {
        CENTRAL.clone()
    }

    pub fn global_executor() -> Self {
        GLOBAL_EXEC.clone()
    }

    pub fn global_logger() -> Self {
        GLOBAL_LOGGER.clone()
    }

    pub fn global_registry() -> Self {
        GLOBAL_REGISTRY.clone()
    }

    pub fn local_portal() -> Self {
        LOCAL_PORTAL.clone()
    }

    pub fn local_star() -> Self {
        LOCAL_STAR.clone()
    }

    pub fn local_hypergate() -> Self {
        LOCAL_HYPERGATE.clone()
    }

    pub fn local_endpoint() -> Self {
        LOCAL_ENDPOINT.clone()
    }

    pub fn remote_endpoint() -> Self {
        REMOTE_ENDPOINT.clone()
    }

    pub fn hyperuser() -> Self {
        HYPERUSER.clone()
    }

    pub fn hyper_userbase() -> Self {
        HYPER_USERBASE.clone()
    }

    pub fn anonymous() -> Self {
        ANONYMOUS.clone()
    }

    pub fn normalize(self) -> Result<Point, SpaceErr> {
        if self.is_normalized() {
            return Ok(self);
        }

        if !self
            .segments
            .first()
            .expect("expected first segment")
            .is_normalized()
        {
            return Err(format!("absolute point paths cannot begin with '..' (reference parent segment) because there is no working point segment: '{}'",self.to_string()).into());
        }

        let mut segments = vec![];
        for seg in &self.segments {
            match seg.is_normalized() {
                true => segments.push(seg.clone()),
                false => {
                    if segments.pop().is_none() {
                        return Err(format!(
                            "'..' too many pop segments directives: out of parents: '{}'",
                            self.to_string()
                        )
                        .into());
                    }
                }
            }
        }
        Ok(Point {
            route: self.route,
            segments,
        })
    }

    pub fn is_parent(&self, child: &Point) -> Result<(), ()> {
        if self.route != child.route {
            return Err(());
        }

        if self.segments.len() >= child.segments.len() {
            return Err(());
        }

        for (index, seg) in self.segments.iter().enumerate() {
            if *seg != *child.segments.get(index).unwrap() {
                return Err(());
            }
        }

        Ok(())
    }

    pub fn is_normalized(&self) -> bool {
        for seg in &self.segments {
            if !seg.is_normalized() {
                return false;
            }
        }
        true
    }

    pub fn to_bundle(self) -> Result<Point, SpaceErr> {
        if self.segments.is_empty() {
            return Err("Point does not contain a bundle".into());
        }

        if let Some(PointSeg::Version(_)) = self.segments.last() {
            return Ok(self);
        }

        return self.parent().expect("expected parent").to_bundle();
    }

    pub fn has_bundle(&self) -> bool {
        if self.segments.is_empty() {
            return false;
        }

        if let Some(PointSeg::Version(_)) = self.segments.last() {
            return true;
        }

        return self.parent().expect("expected parent").to_bundle().is_ok();
    }

    pub fn to_safe_filename(&self) -> String {
        self.to_string()
    }

    pub fn has_filesystem(&self) -> bool {
        for segment in &self.segments {
            match segment {
                PointSeg::FilesystemRootDir => {
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    pub fn is_artifact_bundle_part(&self) -> bool {
        for segment in &self.segments {
            if segment.is_version() {
                return true;
            }
        }
        return false;
    }

    pub fn is_artifact(&self) -> bool {
        if let Option::Some(segment) = self.last_segment() {
            if self.is_artifact_bundle_part() && segment.is_file() {
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn is_artifact_bundle(&self) -> bool {
        if let Option::Some(segment) = self.last_segment() {
            segment.is_version()
        } else {
            false
        }
    }

    pub fn pop(&self) -> Self {
        let mut segments = self.segments.clone();
        segments.pop();
        Point {
            route: self.route.clone(),
            segments,
        }
    }
    pub fn push<S: ToString>(&self, segment: S) -> Result<Self, SpaceErr> {
        let segment = segment.to_string();
        if self.segments.is_empty() {
            let mut point = Self::from_str(segment.as_str())?;
            point.route = self.route.clone();
            Ok(point)
        } else {
            let last = self.last_segment().expect("expected last segment");
            let point = match last {
                PointSeg::Root => segment,
                PointSeg::Space(_) => {
                    format!("{}:{}", self.to_string(), segment)
                }
                PointSeg::Base(_) => {
                    format!("{}:{}", self.to_string(), segment)
                }
                PointSeg::FilesystemRootDir => {
                    format!("{}{}", self.to_string(), segment)
                }
                PointSeg::Dir(_) => {
                    format!("{}{}", self.to_string(), segment)
                }
                PointSeg::Version(_) => {
                    if segment.starts_with(":") {
                        format!("{}{}", self.to_string(), segment)
                    } else {
                        format!("{}:{}", self.to_string(), segment)
                    }
                }
                PointSeg::File(_) => return Err("cannot append to a file".into()),
            };

            let mut point = Self::from_str(point.as_str())?;
            point.route = self.route.clone();
            Ok(point)
        }
    }

    pub fn push_file(&self, segment: String) -> Result<Self, SpaceErr> {
        Self::from_str(format!("{}{}", self.to_string(), segment).as_str())
    }

    pub fn push_segment(&self, segment: PointSeg) -> Result<Self, SpaceErr> {
        if (self.has_filesystem() && segment.is_filesystem_seg()) || segment.kind().is_mesh_seg() {
            let mut point = self.clone();
            point.segments.push(segment);
            Ok(point)
        } else {
            if self.has_filesystem() {
                Err("cannot push a Mesh segment onto a point after the FileSystemRoot segment has been pushed".into())
            } else {
                Err("cannot push a FileSystem segment onto a point until after the FileSystemRoot segment has been pushed".into())
            }
        }
    }

    pub fn filepath(&self) -> Option<String> {
        let mut path = String::new();
        for segment in &self.segments {
            match segment {
                PointSeg::FilesystemRootDir => {
                    path.push_str("/");
                }
                PointSeg::Dir(dir) => {
                    path.push_str(dir.as_str());
                }
                PointSeg::File(file) => {
                    path.push_str(file.as_str());
                }
                _ => {}
            }
        }
        if path.is_empty() {
            None
        } else {
            Some(path)
        }
    }

    pub fn is_filesystem_ref(&self) -> bool {
        if let Option::Some(last_segment) = self.last_segment() {
            last_segment.is_filesystem_seg()
        } else {
            false
        }
    }

    pub fn truncate(self, kind: PointSegKind) -> Result<Point, SpaceErr> {
        let mut segments = vec![];
        for segment in &self.segments {
            segments.push(segment.clone());
            if segment.kind() == kind {
                return Ok(Self {
                    route: self.route,
                    segments,
                });
            }
        }

        Err(SpaceErr::Status {
            status: 404,
            message: format!(
                "Point segment kind: {} not found in point: {}",
                kind.to_string(),
                self.to_string()
            ),
        })
    }
}

impl FromStr for Point {
    type Err = SpaceErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        consume_point(s)
    }
}

impl FromStr for PointVar {
    type Err = SpaceErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        result(point_var(new_span(s)))
    }
}

impl FromStr for PointCtx {
    type Err = SpaceErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(result(point_var(new_span(s)))?.collapse()?)
    }
}

impl Into<String> for Point {
    fn into(self) -> String {
        self.to_string()
    }
}

impl<Route, Seg> PointDef<Route, Seg>
where
    Route: ToString,
    Seg: PointSegQuery + ToString,
{
    pub fn to_string_impl(&self, show_route: bool) -> String {
        let mut rtn = String::new();

        if show_route {
            rtn.push_str(self.route.to_string().as_str());
            rtn.push_str("::");
        }

        let mut post_fileroot = false;

        if self.segments.is_empty() {
            rtn.push_str("ROOT");
            rtn.to_string()
        } else {
            for (i, segment) in self.segments.iter().enumerate() {
                if segment.is_filesystem_root() {
                    post_fileroot = true;
                }
                if i > 0 {
                    rtn.push_str(segment.kind().preceding_delim(post_fileroot));
                }
                rtn.push_str(segment.to_string().as_str());
            }
            rtn.to_string()
        }
    }

    pub fn postfix(&self) -> String {
        self.to_string_impl(false)
    }
}

impl<Route, Seg> ToString for PointDef<Route, Seg>
where
    Route: RouteSegQuery + ToString,
    Seg: PointSegQuery + ToString,
{
    fn to_string(&self) -> String {
        self.to_string_impl(!self.route.is_local())
    }
}

impl Point {
    pub fn root() -> Self {
        Self {
            route: RouteSeg::This,
            segments: vec![],
        }
    }

    pub fn root_with_route(route: RouteSeg) -> Self {
        Self {
            route,
            segments: vec![],
        }
    }

    pub fn is_local_root(&self) -> bool {
        self.segments.is_empty() && self.route.is_local()
    }
}

impl PointVar {
    pub fn is_dir(&self) -> bool {
        self.segments
            .last()
            .unwrap_or(&PointSegVar::Root)
            .kind()
            .is_dir()
    }
}

impl PointCtx {
    pub fn is_dir(&self) -> bool {
        self.segments
            .last()
            .unwrap_or(&PointSegCtx::Root)
            .kind()
            .is_dir()
    }
}

/// A Point is an address usually referencing a Particle.
/// Points can be created from a String composed of ':' delimited segments: `space.com:base:etc`
/// To create a Point:
/// ```
/// use std::str::FromStr;
/// use cosmic_space::loc::Point;
/// let Point = Point::from_str("my-domain.com:apps:my-app")?;
/// ```
/// Besides PointSegs points also have a RouteSeg which can change the meaning of a Point drastically
/// including referencing a completely different Cosmos, etc.  Routes are prepended and delimited by
/// a `::` i.e. `GLOBAL::executor:service`
///
pub type Point = PointDef<RouteSeg, PointSeg>;

/// A Point with potential contextual information for example one with a working dir:
/// `.:mechtrons:mechtron`  the single `.` works the same as in unix shell and refers to the `working`
/// location.  You can also reference parent hierarchies just as you would expect: `..:another-app:something`
///
/// In order to create an absolute Point from a PointCtx one must call the PointCtx::to_resolved(&env) method
/// with a proper Env (environment) reference which should have a contextual point set:
/// ```
/// use std::str::FromStr;
/// use cosmic_space::loc::Point;
/// use cosmic_space::loc::PointCtx;
/// let point_var = PointCtx::from_str("..:another-app:something")?;
/// let point: Point = point_ctx.to_resolve(&env)?;
/// ```
pub type PointCtx = PointDef<RouteSeg, PointSegCtx>;

/// A Point with potential Variables and Context (see PointCtx)
/// this point may look like `my-domain:users:${user}` in which case before it can be made into a
/// usable point it must be resolved like so:
/// ```
/// use std::str::FromStr;
/// use cosmic_space::loc::{Point, PointVar};
/// let point_var = PointVar::from_str("my-domain:users:${user}")?;
/// let point: Point = point_var.to_resolve(&env)?;
/// ```
pub type PointVar = PointDef<RouteSegVar, PointSegVar>;


#[cfg(test)]
pub mod test {
    use core::str::FromStr;
    use crate::point::Point;

    #[test]
    pub fn test_retain_route() {
        let users = Point::from_str("HYPER::users").unwrap();
        let less = users.push("less".to_string()).unwrap();

        assert_eq!("HYPER::users:less", less.to_string().as_str())
    }
}