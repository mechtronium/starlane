use crate::lib::std::boxed::Box;
use crate::lib::std::string::ToString;
use crate::lib::std::vec::Vec;
use core::ops::Deref;
use nom_supreme::context::ContextError;
use thiserror_no_std::Error;
use starlane_primitive_macros::{AsStr, Autobox};
use crate::space::parse::block::BlockKind;
use crate::space::parse::nomplus::Input;


pub trait ToInputCtx  {
    fn to(self) -> impl Fn()->InputCtx;
}

#[derive(Copy,Clone,Debug,Error,strum_macros::IntoStaticStr)]
pub enum InputCtx {
 #[error("{0}")]
 Prim(PrimCtx),
 #[error("{0}")]
 Case(CaseCtx),
 #[error("{0}")]
 Point(PointCtx),
 #[error("{0}")]
 Block(BlockKind)
}

impl ToInputCtx for InputCtx {
    fn to(self) -> impl Fn()->InputCtx
    {
        move || self
    }
}

#[derive(Copy,Clone,Debug,Error,strum_macros::IntoStaticStr)]
pub enum PrimCtx {
    #[error("token")]
    Token,
    #[error("lex")]
    Lex,
}

impl ToInputCtx for PrimCtx{
    fn to(self) -> impl Fn()->InputCtx
    {
        move || InputCtx::Prim(self)
    }
}


#[derive(Copy,Clone,Debug,Error)]
pub enum CaseCtx {
    #[error("expected skewer case value (lowercase alphanumeric & '-')")]
    SkewerCase,
    #[error("expected CamelCase name (mixed case alphanumeric)")]
    CamelCase,
    #[error("expected variable case name (lowercase alphanumeric & '_')")]
    VarCase,
    #[error("expected filename (mixed case alphanumeric & '_' & '-')")]
    FileCase,
    #[error("expected domain case (mixed case alphanumeric & '-' & '.' )")]
    DomainCase,
    #[error("expected filename (mixed case alphanumeric & '_' & '-') must end with a '/'")]
    DirCase,
}



impl ToInputCtx for CaseCtx{
    fn to(self) -> impl Fn()->InputCtx
    {
        move || InputCtx::Case(self)
    }
}




#[derive(Copy,Clone,Error,Debug)]
pub enum PointCtx {
    #[error("Var def")]
    Var,
    #[error("RouteSeg")]
    RouteSeg,
    #[error("BasePointSeg")]
    BaseSeg
}

impl ToInputCtx for PointCtx{
    fn to(self) -> impl Fn()->InputCtx
    {
        move || InputCtx::Point(self)
    }
}




pub type Stack = Vec<InputCtx>;



