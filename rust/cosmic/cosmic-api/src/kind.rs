use crate::id::id::Version;
use crate::parse::{CamelCase, Domain, SkewerCase};
use crate::selector::selector::VersionReq;
use http::uri::Parts;
use serde::{Deserialize, Serialize};
use strum::ParseError::VariantNotFound;
use crate::MsgErr;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct SubTypeDef<Part, SubType> {
    pub part: Part,
    pub sub: SubType,
    pub r#type: SubType,
}

impl<Part, SubType, IsMatchPart, IsMatchSubType> IsMatch<SubTypeDef<Part, SubType>>
    for SubTypeDef<IsMatchPart, IsMatchSubType>
where
    IsMatchPart: IsMatch<Part>,
    IsMatchSubType: IsMatch<SubType>,
    Part: Eq + PartialEq,
    SubType: Eq + PartialEq,
{
    fn is_match(&self, other: &SubTypeDef<Part, SubType>) -> bool {
        self.part.is_match(&other.part)
            && self.sub.is_match(&other.sub)
            && self.r#type.is_match(&other.r#type)
    }
}

impl<Part, SubType> SubTypeDef<Part, SubType> {
    pub fn with_sub(self, sub: SubType) -> Self {
        Self {
            part: self.part,
            r#type: self.r#type,
            sub,
        }
    }

    pub fn with_type(self, r#type: SubType) -> Self {
        Self {
            part: self.part,
            sub: self.sub,
            r#type,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct ParentChildDef<Parent, Child> {
    pub parent: Parent,
    pub child: Child,
}

impl<Parent, Child, IsMatchParent, IsMatchChild> IsMatch<ParentChildDef<Parent, Child>>
    for ParentChildDef<IsMatchParent, IsMatchChild>
where
    IsMatchParent: IsMatch<Parent>,
    IsMatchChild: IsMatch<Child>,
    Parent: Eq + PartialEq,
    Child: Eq + PartialEq,
{
    fn is_match(&self, other: &ParentChildDef<Parent, Child>) -> bool {
        self.parent.is_match(&other.parent) && self.child.is_match(&other.child)
    }
}

impl<Parent, Child> Default for ParentChildDef<Parent, Child>
where
    Parent: Default,
    Child: Default,
{
    fn default() -> Self {
        Self {
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, strum_macros::Display)]
pub enum Variant {
    Artifact,
    Db(Db),
}

impl Variant {
    pub fn from( kind: &Kind, variant: &CamelCase ) -> Result<Self,MsgErr> {
        match kind {
            Kind::Db => {
                Ok(Variant::Db(Db::from_str(variant.as_str())?))
            }
            what => Err(format!("kind '{}' does not have a variant '{}' ", kind.to_string(), variant.to_string()).into())
        }
    }
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    strum_macros::Display,
    strum_macros::EnumString,
)]
pub enum Db {
    Rel,
}

impl Variant {
    pub fn to_sub_types(self) -> VariantSubTypes {
        VariantSubTypes {
            part: self,
            sub: None,
            r#type: None,
        }
    }

    pub fn with_specific(self, specific: Option<SpecificSubTypes>) -> VariantFull {
        VariantFull {
            parent: self.to_sub_types(),
            child: specific,
        }
    }
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    strum_macros::Display,
    strum_macros::EnumString,
)]
pub enum Kind {
    Root,
    Space,
    Auth,
    Base,
    Mechtron,
    FileSys,
    Db,
    Artifact,
    Control,
    Portal,
    Star,
    Driver,
    Global,
}

impl Kind {
    pub fn to_sub_types(self) -> KindSubTypes {
        KindSubTypes {
            part: self,
            sub: None,
            r#type: None,
        }
    }

    pub fn with_variant(self, variant: Option<VariantFull>) -> KindFull {
        KindFull {
            parent: self.to_sub_types(),
            child: variant,
        }
    }
}

impl Default for Kind {
    fn default() -> Self {
        Self::Root
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct SpecificDef<Domain, Skewer, Version> {
    pub provider: Domain,
    pub vendor: Domain,
    pub product: Skewer,
    pub variant: Skewer,
    pub version: Version,
}


pub type Specific = SpecificDef<Domain, SkewerCase, Version>;


impl Specific {
    pub fn new(
        provider: Domain,
        vendor: Domain,
        product: SkewerCase,
        variant: SkewerCase,
        version: Version,
    ) -> Self {
        Self {
            provider,
            vendor,
            product,
            variant,
            version,
        }
    }

    pub fn to_full(self) -> SpecificSubTypes {
        SpecificSubTypes {
            part: self,
            sub: None,
            r#type: None,
        }
    }

    pub fn sub(self, sub: Option<CamelCase>) -> SpecificSubTypes {
        SpecificSubTypes {
            part: self,
            sub,
            r#type: None,
        }
    }

    pub fn sub_type(self, sub: Option<CamelCase>, r#type: Option<CamelCase>) -> SpecificSubTypes {
        SpecificSubTypes {
            part: self,
            sub,
            r#type,
        }
    }
}
pub type VariantSubTypes = MatcherDef<Variant, Option<CamelCase>>;

pub type SpecificSubTypes = MatcherDef<Specific, Option<CamelCase>>;

pub type VariantDef<Variant,Specific,Camel>= ParentMatcherDef<Variant, Specific, Camel>;
pub type VariantFull = VariantDef<Variant, Option<SpecificSubTypes>, Option<CamelCase>>;
pub type ProtoVariant= VariantDef<CamelCase, Option<SpecificSubTypes>, Option<CamelCase>>;
pub type KindDef<Kind,Variant,Camel>= ParentMatcherDef<Kind,Variant,Camel>;
pub type CamelCaseSubTypes = MatcherDef<CamelCase, Option<CamelCase>>;
pub type KindSubTypes = MatcherDef<Kind, Option<CamelCase>>;
pub type KindFull = KindDef<Kind, Option<VariantFull>, Option<CamelCase>>;
pub type ProtoKind = KindDef<CamelCase, Option<ProtoVariant>, Option<CamelCase>>;

pub type MatcherDef<Matcher, SubTypeMatcher> = SubTypeDef<Matcher, SubTypeMatcher>;
pub type ParentMatcherDef<Matcher, Child, SubTypeMatcher> =
    ParentChildDef<SubTypeDef<Matcher, SubTypeMatcher>, Child>;

pub trait IsMatch<X>
where
    X: Eq + PartialEq,
{
    fn is_match(&self, other: &X) -> bool;
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum Pattern<X> {
    None,
    Any,
    Matches(X),
}

impl<X> IsMatch<X> for Pattern<X>
where
    X: Eq + PartialEq,
{
    fn is_match(&self, other: &X) -> bool {
        match self {
            Pattern::None => false,
            Pattern::Any => true,
            Pattern::Matches(x) => x.eq(other),
        }
    }
}

impl<X> ToString for Pattern<X>
where
    X: ToString,
{
    fn to_string(&self) -> String {
        match self {
            Pattern::None => "!".to_string(),
            Pattern::Any => "*".to_string(),
            Pattern::Matches(x) => x.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum OptPattern<X> {
    None,
    Any,
    Matches(X),
}

impl<X> IsMatch<Option<X>> for OptPattern<X>
where
    X: Eq + PartialEq,
{
    fn is_match(&self, other: &Option<X>) -> bool {
        match self {
            Self::None => other.is_none(),
            Self::Any => true,
            Self::Matches(x) => match other {
                None => false,
                Some(o) => *x == *o,
            },
        }
    }
}

impl<X> ToString for OptPattern<X>
where
    X: ToString,
{
    fn to_string(&self) -> String {
        match self {
            Self::None => "!".to_string(),
            Self::Any => "*".to_string(),
            Self::Matches(x) => x.to_string(),
        }
    }
}

impl IsMatch<Version> for VersionReq {
    fn is_match(&self, other: &Version) -> bool {
        self.version.matches(&other.version)
    }
}

pub type DomainSelector = Pattern<Domain>;
pub type SkewerSelector = Pattern<SkewerCase>;
pub type VersionSelector = Pattern<VersionReq>;
pub type SpecificSelector = SpecificDef<DomainSelector, SkewerSelector, VersionSelector>;
pub type SpecificFullSelector = MatcherDef<SpecificSelector, OptPattern<CamelCase>>;

impl SpecificSelector {
    pub fn to_full(self) -> SpecificFullSelector {
        SpecificFullSelector {
            part: self,
            sub: OptPattern::None,
            r#type: OptPattern::None,
        }
    }
}

impl IsMatch<Specific> for SpecificSelector {
    fn is_match(&self, other: &Specific) -> bool {
        self.provider.is_match(&other.provider)
            && self.vendor.is_match(&other.vendor)
            && self.product.is_match(&other.product)
            && self.variant.is_match(&other.variant)
    }
}

pub type VariantFullSelector =
    ParentMatcherDef<Pattern<Variant>, OptPattern<SpecificSubTypes>, OptPattern<CamelCase>>;
pub type KindFullSelector =
    ParentMatcherDef<Pattern<Kind>, OptPattern<VariantFullSelector>, OptPattern<CamelCase>>;

pub mod parse {
    use crate::kind::{Kind, OptPattern, ParentChildDef, Pattern, Specific, SpecificDef, SpecificFullSelector, SpecificSelector, SpecificSubTypes, SubTypeDef, Variant, VariantFull};
    use crate::parse::{camel_case, domain, skewer_case, version, version_req, Domain, CamelCase};
    use cosmic_nom::{Res, Span};
    use nom::branch::alt;
    use nom::bytes::complete::tag;
    use nom::combinator::{fail, opt, success, value};
    use nom::sequence::{delimited, pair, preceded, tuple};
    use std::str::FromStr;

    pub fn pattern<I, FnX, X>(mut f: FnX) -> impl FnMut(I) -> Res<I, Pattern<X>> + Copy
    where
        I: Span,
        FnX: FnMut(I) -> Res<I, X> + Copy,
        X: Clone,
    {
        move |input| {
            alt((
                value(Pattern::Any, tag("*")),
                value(Pattern::None, tag("!")),
                |i| f(i).map(|(next, x)| (next, Pattern::Matches(x))),
            ))(input)
        }
    }

    pub fn opt_pattern<I, FnX, X>(mut f: FnX) -> impl FnMut(I) -> Res<I, OptPattern<X>> + Copy
    where
        I: Span,
        FnX: FnMut(I) -> Res<I, X> + Copy,
        X: Clone,
    {
        move |input| {
            alt((
                value(OptPattern::Any, tag("*")),
                value(OptPattern::None, tag("!")),
                |i| f(i).map(|(next, x)| (next, OptPattern::Matches(x))),
            ))(input)
        }
    }

    pub fn preceded_opt_pattern<I, FnX, X, FnPrec>(
        prec: FnPrec,
        mut f: FnX,
    ) -> impl FnMut(I) -> Res<I, OptPattern<X>> + Copy
    where
        I: Span,
        FnX: FnMut(I) -> Res<I, X> + Copy,
        X: Clone,
        FnPrec: FnMut(I) -> Res<I, I> + Copy,
    {
        move |input| alt((preceded(prec, opt_pattern(f)), |i|Ok((i,OptPattern::None))))(input)
    }

    fn sub_types<I, FnPart, Part, FnCamel, Camel>(
        fn_part: FnPart,
        fn_camel: FnCamel,
    ) -> impl FnMut(I) -> Res<I, SubTypeDef<Part, Camel>>
    where
        FnPart: FnMut(I) -> Res<I, Part> + Copy,
        FnCamel: FnMut(I) -> Res<I, Camel> + Copy,
        I: Span,
    {
        move |input: I| {
            tuple((fn_part, fn_camel, fn_camel))(input)
                .map(|(next, (part, sub, r#type))| (next, SubTypeDef { part, sub, r#type }))
        }
    }

    fn parent_child_def<I, FnParent, Parent, FnChild, Child>(
        fn_parent: FnParent,
        fn_child: FnChild,
    ) -> impl FnMut(I) -> Res<I, ParentChildDef<Parent,Child>>
        where
            FnParent: FnMut(I) -> Res<I, Parent> + Copy,
            FnChild: FnMut(I) -> Res<I, Child> + Copy,
            I: Span,
    {
        move |input: I| {
            pair(fn_parent,fn_child)(input).map( |(next,(parent,child))|{
                (next,
                ParentChildDef{
                    parent,
                    child
                })
            })
        }
    }


    pub fn specific_def<I, FnDomain, FnSkewer, FnVersion, Domain, Skewer, Version>(
        fn_domain: FnDomain,
        fn_skewer: FnSkewer,
        fn_version: FnVersion,
    ) -> impl FnMut(I) -> Res<I, SpecificDef<Domain, Skewer, Version>> + Copy
    where
        I: Span,
        FnDomain: FnMut(I) -> Res<I, Domain> + Copy,
        FnSkewer: FnMut(I) -> Res<I, Skewer> + Copy,
        FnVersion: FnMut(I) -> Res<I, Version> + Copy,
    {
        move |input: I| {
            tuple((
                fn_domain,
                tag(":"),
                fn_domain,
                tag(":"),
                fn_skewer,
                tag(":"),
                fn_skewer,
                tag(":"),
                fn_version,
            ))(input)
            .map(
                |(next, (provider, _, vendor, _, product, _, variant, _, version))| {
                    (
                        next,
                        SpecificDef {
                            provider,
                            vendor,
                            product,
                            variant,
                            version,
                        },
                    )
                },
            )
        }
    }

    pub fn specific<I>(input: I) -> Res<I, Specific>
    where
        I: Span,
    {
        specific_def(domain, skewer_case, version)(input)
    }

    pub fn specific_sub_types<I>(input: I) -> Res<I, SpecificSubTypes>
    where
        I: Span,
    {
        sub_types(specific, |i| opt(preceded(tag(":"), camel_case))(i))(input)
    }

    pub fn specific_selector<I>(input: I) -> Res<I, SpecificSelector>
    where
        I: Span,
    {
        specific_def(pattern(domain), pattern(skewer_case), pattern(version_req))(input)
    }

    pub fn specific_full_selector<I>(input: I) -> Res<I, SpecificFullSelector>
    where
        I: Span,
    {
        sub_types(
            specific_selector,
            preceded_opt_pattern(|i|tag(":")(i),camel_case )
//                  preceded_opt_pattern(|i|tag(":")(i), camel_case),
        )(input)
    }

    /*
    pub fn variant_def<I,Kind,FnVariant,FnSelector,Variant,Selector>( kind: Kind, fn_variant: FnVariant, fn_selector: FnSelector ) -> impl Res<I,VariantFull>

     */


    #[cfg(test)]
    pub mod test {
        use crate::kind::parse::{opt_pattern, preceded_opt_pattern, specific, specific_full_selector, specific_selector, specific_sub_types};
        use crate::parse::error::result;
        use crate::parse::{camel_case, CamelCase};
        use crate::util::log;
        use core::str::FromStr;
        use nom::bytes::complete::tag;
        use nom::combinator::all_consuming;
        use nom::sequence::preceded;
        use cosmic_nom::new_span;
        use crate::id::id::Version;
        use crate::kind::{OptPattern, Pattern};
        use crate::selector::selector::VersionReq;

        #[test]
        pub fn test_my_sub() {
            let sub = log(result(opt_pattern(camel_case )( new_span("MySub")))).unwrap();
            assert_eq!(sub, OptPattern::Matches(CamelCase::from_str("MySub").unwrap()));

            let sub = log(result(preceded_opt_pattern(|i|tag(":")(i),camel_case )( new_span(":MySub")))).unwrap();
            assert_eq!(sub, OptPattern::Matches(CamelCase::from_str("MySub").unwrap()));
        }

        #[test]
        pub fn test_specific() {
            let specific = result(specific(new_span(
                "my-domain.io:vendor.io:product:variant:1.0.0",
            )))
            .unwrap();
        }

        #[test]
        pub fn test_specific_selector() {
            let selector = log(result(specific_selector(new_span(
                "my-domain.io:*:product:variant:1.0.0",
            ))))
            .unwrap();
        }

        #[test]
        pub fn test_specific_sub_types() {
            let specific = result(specific_sub_types(new_span(
                "my-domain.io:vendor.io:product:variant:1.0.0:Sub:Type",
            )))
            .unwrap();
            assert_eq!(specific.sub, Some(CamelCase::from_str("Sub").unwrap()));
            assert_eq!(specific.r#type, Some(CamelCase::from_str("Type").unwrap()));
        }

        #[test]
        pub fn test_specific_full_selector() {
            let selector = log(result(specific_full_selector(new_span(
                "my-domain.io:*:product:variant:1.0.0",
            ))))
            .unwrap();
            assert_eq!(selector.sub, OptPattern::None);
            assert_eq!(selector.part.variant.to_string(),"variant".to_string());
//            assert_eq!(selector.part.version,Pattern::Matches(VersionReq::from_str("1.0.0").unwrap()));

            let selector = log(result(specific_full_selector(new_span(
                "my-domain.io:*:product:variant:1.0.0:MySub",
            ))))
                .unwrap();

            assert_eq!(selector.sub, OptPattern::Matches(CamelCase::from_str("MySub").unwrap()));
        }
    }
}

#[cfg(test)]
pub mod test {
    use crate::id::id::Version;
    use crate::kind::{
        DomainSelector, IsMatch, Kind, OptPattern, ParentChildDef, Pattern, SkewerSelector,
        Specific, SpecificSelector, SpecificSubTypes, SubTypeDef, Variant, VariantFull,
        VariantFullSelector, VersionSelector,
    };
    use crate::parse::{CamelCase, Domain, SkewerCase};
    use crate::selector::selector::VersionReq;
    use core::str::FromStr;

    fn create_specific() -> Specific {
        Specific::new(
            Domain::from_str("my-domain.com").unwrap(),
            Domain::from_str("my-domain.com").unwrap(),
            SkewerCase::from_str("product").unwrap(),
            SkewerCase::from_str("variant").unwrap(),
            Version::from_str("1.0.0").unwrap(),
        )
    }

    fn create_specific_sub_type() -> SpecificSubTypes {
        create_specific().sub(Some(CamelCase::from_str("Blah").unwrap()))
    }

    fn create_variant_full() -> VariantFull {
        Variant::Artifact.with_specific(Some(create_specific_sub_type()))
    }

    #[test]
    pub fn specific() {
        let specific1 = create_specific();
        let specific2 = create_specific();
        assert_eq!(specific1, specific2);

        let spec1 = create_specific_sub_type();
        let spec2 = create_specific_sub_type();
        assert_eq!(spec1, spec2);
    }

    #[test]
    pub fn variant() {
        let var1 = Variant::Artifact.with_specific(Some(create_specific_sub_type()));
        let var2 = Variant::Artifact.with_specific(Some(create_specific_sub_type()));
        assert_eq!(var1, var2);
    }

    #[test]
    pub fn kind() {
        let kind1 = Kind::Root.with_variant(Some(create_variant_full()));
        let kind2 = Kind::Root.with_variant(Some(create_variant_full()));
        assert_eq!(kind1, kind2);
    }

    #[test]
    pub fn specific_selector() {
        let specific = create_specific();
        let selector = SpecificSelector {
            provider: DomainSelector::Any,
            vendor: DomainSelector::Matches(Domain::from_str("my-domain.com").unwrap()),
            product: SkewerSelector::Any,
            variant: SkewerSelector::Matches(SkewerCase::from_str("variant").unwrap()),
            version: VersionSelector::Matches(VersionReq::from_str("^1.0.0").unwrap()),
        };

        assert!(selector.is_match(&specific));

        let mut specific = specific.to_full();
        let mut selector = selector.to_full();

        assert!(selector.is_match(&specific));

        let specific = specific.with_sub(Some(CamelCase::from_str("Zophis").unwrap()));
        assert!(!selector.is_match(&specific));
        let selector = selector.with_sub(OptPattern::Any);
        assert!(selector.is_match(&specific));

        let selector = SpecificSelector {
            provider: DomainSelector::None,
            vendor: DomainSelector::Matches(Domain::from_str("my-domain.com").unwrap()),
            product: SkewerSelector::Any,
            variant: SkewerSelector::Matches(SkewerCase::from_str("variant").unwrap()),
            version: VersionSelector::Matches(VersionReq::from_str("^1.0.0").unwrap()),
        };

        let specific = create_specific();

        assert!(!selector.is_match(&specific));
    }

    #[test]
    pub fn variant_selector() {
        let variant = create_variant_full();
        let mut selector = VariantFullSelector {
            parent: SubTypeDef {
                part: Pattern::Matches(Variant::Artifact),
                sub: OptPattern::None,
                r#type: OptPattern::None,
            },
            child: OptPattern::None,
        };

        assert!(!selector.is_match(&variant));

        let mut selector = VariantFullSelector {
            parent: SubTypeDef {
                part: Pattern::Matches(Variant::Artifact),
                sub: OptPattern::None,
                r#type: OptPattern::None,
            },
            child: OptPattern::Any,
        };

        assert!(selector.is_match(&variant));
    }
}
