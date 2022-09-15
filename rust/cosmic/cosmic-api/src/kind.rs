use crate::id::id::Version;
use crate::parse::{CamelCase, Domain, SkewerCase};
use http::uri::Parts;
use serde::{Deserialize, Serialize};
use strum::ParseError::VariantNotFound;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct PartSubTypeDef<Part, SubTypeMatcher> {
    pub part: Part,
    pub sub: SubTypeMatcher,
    pub r#type: SubTypeMatcher,
}

impl <Part, SubTypeMatcher> PartSubTypeDef<Part, SubTypeMatcher> {
    pub fn with_sub( self, sub: SubTypeMatcher) -> Self {
        Self {
            part: self.part,
            r#type: self.r#type,
            sub
        }
    }

    pub fn with_type( self, r#type: SubTypeMatcher) -> Self {
        Self {
            part: self.part,
            sub: self.sub,
            r#type
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct ParentChildDef<Parent, Child> {
    pub parent: Parent,
    pub child: Child,
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
pub enum Variant {
    Artifact,
}

impl Variant {
    pub fn to_sub_types(self) -> VariantSubTypes {
        VariantSubTypes {
            part: self,
            sub: None,
            r#type: None
        }
    }

    pub fn with_specific( self, specific: Option<SpecificFull> ) -> VariantFull {
        VariantFull {
            parent: self.to_sub_types(),
            child: specific
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
            r#type: None
        }
    }

    pub fn with_variant( self, variant: Option<VariantFull> ) -> KindFull {
        KindFull {
            parent: self.to_sub_types(),
            child: variant
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

    pub fn sub(self, sub: Option<CamelCase>) -> SpecificFull {
        SpecificFull {
            part: self,
            sub,
            r#type: None,
        }
    }

    pub fn sub_type(self, sub: Option<CamelCase>, r#type: Option<CamelCase>) -> SpecificFull {
        SpecificFull {
            part: self,
            sub,
            r#type,
        }
    }
}

pub type SpecificFull = MatcherDef<Specific, Option<CamelCase>>;
pub type VariantSubTypes= MatcherDef<Variant, Option<CamelCase>>;
pub type VariantFull = ParentMatcherDef<Variant, Option<SpecificFull>, Option<CamelCase>>;
pub type KindSubTypes= MatcherDef<Kind, Option<CamelCase>>;
pub type KindFull = ParentMatcherDef<Kind, Option<VariantFull>, Option<CamelCase>>;

pub type MatcherDef<Matcher, SubTypeMatcher> = PartSubTypeDef<Matcher, SubTypeMatcher>;
pub type ParentMatcherDef<Matcher,Child,SubTypeMatcher> = ParentChildDef<PartSubTypeDef<Matcher, SubTypeMatcher>,Child>;


#[cfg(test)]
pub mod test {
    use crate::id::id::Version;
    use crate::parse::{CamelCase, Domain, SkewerCase};
    use core::str::FromStr;
    use crate::kind::{Kind, Specific, SpecificFull, Variant, VariantFull};

    fn create_specific() -> Specific {
        Specific::new(
            Domain::from_str("my-domain.com").unwrap(),
            Domain::from_str("my-domain.com").unwrap(),
            SkewerCase::from_str("product").unwrap(),
            SkewerCase::from_str("variant").unwrap(),
            Version::from_str("1.0.0").unwrap(),
        )
    }

    fn create_specific_sub_type() -> SpecificFull {
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
        assert_eq!(var1,var2);
    }

    #[test]
    pub fn kind() {
        let kind1 = Kind::Root.with_variant(Some(create_variant_full()));
        let kind2=  Kind::Root.with_variant(Some(create_variant_full()));
        assert_eq!(kind1,kind2);
    }

}
