use std::fmt;
use std::str::FromStr;

use bincode::deserialize;
use serde::{Deserialize, Serialize, Serializer};
use uuid::Uuid;

use crate::actor::{Actor, ActorKey, ActorKind};
use crate::app::{AppKind};
use crate::artifact::{Artifact, ArtifactKey, ArtifactKind};
use crate::error::Error;
use crate::filesystem::FileKey;
use crate::frame::Reply;
use crate::id::Id;
use crate::message::Fail;
use crate::names::Name;
use crate::permissions::{Priviledges, User, UserKind};
use crate::resource::{Labels, ResourceAssign, ResourceType, ResourceManagerKey, Resource, ResourceArchetype, ResourceKind, ResourceAddressPart, Skewer};

#[derive(Clone,Serialize,Deserialize,Hash,Eq,PartialEq)]
pub enum SpaceKey
{
    HyperSpace,
    Space(u32)
}

impl SpaceKey
{

    pub fn from_index(index: u32) -> Self
    {
        if index == 0
        {
            SpaceKey::HyperSpace
        }
        else
        {
            SpaceKey::Space(index)
        }
    }

    pub fn index(&self)->u32
    {
        match self
        {
            SpaceKey::HyperSpace => 0,
            SpaceKey::Space(index) => index.clone()
        }
    }
}

impl fmt::Display for SpaceKey{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!( f,"{}",
                match self{
                    SpaceKey::HyperSpace => "HyperSpace".to_string(),
                    SpaceKey::Space(index) => index.to_string()
                })
    }

}



#[derive(Clone,Serialize,Deserialize,Hash,Eq,PartialEq)]
pub struct UserKey
{
  pub space: SpaceKey,
  pub id: UserId
}

impl UserKey
{
    pub fn bin(&self)->Result<Vec<u8>,Error>
    {
        let mut bin= bincode::serialize(self)?;
        Ok(bin)
    }

    pub fn from_bin(mut bin: Vec<u8> )->Result<Self,Error>
    {
        let mut key = bincode::deserialize::<Self>(bin.as_slice() )?;
        Ok(key)
    }
}



impl UserKey
{
    pub fn new(space: SpaceKey) -> Self
    {
        UserKey{
            space,
            id: UserId::new()
        }
    }

    pub fn with_id(space: SpaceKey, id: UserId) -> Self
    {
        UserKey{
            space,
            id: id
        }
    }

    pub fn hyper_user() -> Self
    {
        UserKey::with_id(SpaceKey::HyperSpace, UserId::Super)
    }


    pub fn super_user(space: SpaceKey) -> Self
    {
        UserKey::with_id(space,UserId::Super)
    }

    pub fn annonymous(space: SpaceKey) -> Self
    {
        UserKey::with_id(space,UserId::Annonymous)
    }


    pub fn is_hyperuser(&self)->bool
    {
        match self.space{
            SpaceKey::HyperSpace => {
                match self.id
                {
                    UserId::Super => true,
                    _ => false
                }
            }
            _ => false
        }
    }

    pub fn privileges(&self) -> Priviledges
    {
        if self.is_hyperuser()
        {
            Priviledges::all()
        }
        else {
            Priviledges::new()
        }
    }
}

impl fmt::Display for UserKey{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!( f,"({},{})",self.space, self.id)
    }

}


#[derive(Clone,Serialize,Deserialize,Hash,Eq,PartialEq)]
pub enum UserId
{
    Super,
    Annonymous,
    Uuid(Uuid)
}

impl UserId
{
    pub fn new()->Self
    {
        Self::Uuid(Uuid::new_v4())
    }
}

impl fmt::Display for UserId{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!( f,"{}",match self{
            UserId::Super => "super".to_string(),
            UserId::Annonymous => "annonymous".to_string(),
            UserId::Uuid(uuid) => uuid.to_string().to_lowercase()
        })
    }

}

#[derive(Clone,Serialize,Deserialize,Eq,PartialEq,Hash)]
pub struct SubSpaceKey
{
    pub space: SpaceKey,
    pub id: SubSpaceId
}

impl SubSpaceKey
{
    pub fn hyper_default( ) -> Self
    {
        SubSpaceKey::new(SpaceKey::HyperSpace, SubSpaceId::Default )
    }

    pub fn new( space: SpaceKey, id: SubSpaceId ) -> Self
    {
        SubSpaceKey{
            space: space,
            id: id
        }
    }
}


impl fmt::Display for SubSpaceKey{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!( f,"{}-{}",self.space, self.id)
    }

}


#[derive(Clone,Serialize,Deserialize,Eq,PartialEq,Hash)]
pub enum SubSpaceId
{
    Default,
    Index(u32)
}

impl SubSpaceId
{
    pub fn from_index(index: u32) -> Self
    {
        if index == 0
        {
            Self::Default
        }
        else
        {
            Self::Index(index)
        }
    }

    pub fn index(&self)->u32
    {
        match self
        {
            SubSpaceId::Default => 0,
            SubSpaceId::Index(index) => index.clone()
        }
    }
}


#[derive(Clone,Hash,Eq,PartialEq,Serialize,Deserialize)]
pub struct AppKey
{
    pub sub_space: SubSpaceKey,
    pub id: AppId
}

impl AppKey {
    pub fn address_part(&self) -> Result<ResourceAddressPart,Error>{
        Ok(ResourceAddressPart::Skewer(Skewer::new(self.id.to_string().as_str() )?))
    }
}



impl AppKey
{
    pub fn bin(&self)->Result<Vec<u8>,Error>
    {
        let mut bin= bincode::serialize(self)?;
        Ok(bin)
    }

    pub fn from_bin(mut bin: Vec<u8> )->Result<AppKey,Error>
    {
        let mut key = bincode::deserialize::<AppKey>(bin.as_slice() )?;
        Ok(key)
    }

}

#[derive(Clone,Hash,Eq,PartialEq,Serialize,Deserialize)]
pub enum AppId
{
    HyperApp,
    Uuid(Uuid)
}

impl AppId {
    pub fn encode(&self) -> Result<String, Error> {
        Ok(base64::encode(self.to_string().as_bytes().to_vec()))
    }

    pub fn decode(string: String) -> Result<Self, Error> {
        Ok(AppId::from_str(String::from_utf8(base64::decode(string)?)?.as_str())?)
    }
}



impl AppId
{
    pub fn new()->Self
    {
        Self::Uuid(Uuid::new_v4())
    }
}



impl AppKey
{
    pub fn new( sub_space: SubSpaceKey )->Self
    {
        AppKey{
            sub_space: sub_space,
            id: AppId::new()
        }
    }
}

impl fmt::Display for AppKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({},{})", self.sub_space, self.id.to_string())
    }
}



impl FromStr for AppId{
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s{
            "hyper-app" => {
                    Ok(AppId::HyperApp)
                }
            _ => Ok(AppId::Uuid(Uuid::from_str(s)?))
        }
    }
}


impl ToString for AppId{

    fn to_string(&self) -> String {
        match self
        {
            AppId::HyperApp => "hyper-app".to_string(),
            AppId::Uuid(uuid) => uuid.to_string().to_lowercase()
        }
    }
}


impl fmt::Display for SubSpaceId{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self
        {
            SubSpaceId::Default => "Default".to_string(),
            SubSpaceId::Index(index) => index.to_string()
        };
        write!(f, "{}", str )
    }
}

pub type MessageId = Uuid;

#[derive(Clone,Serialize,Deserialize,Hash,Eq,PartialEq)]
pub enum ResourceKey
{
    Space(SpaceKey),
    SubSpace(SubSpaceKey),
    App(AppKey),
    Actor(ActorKey),
    User(UserKey),
    Artifact(ArtifactKey),
    File(FileKey),
    FileSystem(FileSystemKey),
}

impl ResourceKey
{

    pub fn parent(&self)->Option<ResourceKey>{
        match self {
            ResourceKey::Space(_) => Option::None,
            ResourceKey::SubSpace(sub_space) => Option::Some(ResourceKey::Space(sub_space.space.clone())),
            ResourceKey::App(app) =>  Option::Some(ResourceKey::SubSpace(app.sub_space.clone())),
            ResourceKey::Actor(actor) =>  Option::Some(ResourceKey::App(actor.app.clone())),
            ResourceKey::User(user) => Option::Some(ResourceKey::Space(user.space.clone())),
            ResourceKey::Artifact(artifact) => Option::Some(ResourceKey::SubSpace(artifact.sub_space.clone())),
            ResourceKey::File(file) => Option::Some(ResourceKey::FileSystem(file.filesystem.clone())),
            ResourceKey::FileSystem(filesystem) => {
                match filesystem{
                    FileSystemKey::App(app) => {
                        Option::Some(ResourceKey::App(app.app.clone()))
                    }
                    FileSystemKey::SubSpace(sub_space) => {
                        Option::Some(ResourceKey::SubSpace(sub_space.sub_space.clone()))
                    }
                }
            }
        }
    }

    pub fn space(&self)->SpaceKey {
        match self{
            ResourceKey::Space(space) => space.clone(),
            ResourceKey::SubSpace(sub_space) => sub_space.space.clone(),
            ResourceKey::App(app) => app.sub_space.space.clone(),
            ResourceKey::Actor(actor) => actor.app.sub_space.space.clone(),
            ResourceKey::User(user) => user.space.clone(),
            ResourceKey::Artifact(artifact) => artifact.sub_space.space.clone(),
            ResourceKey::File(file) => match &file.filesystem{
                FileSystemKey::App(app) => app.app.sub_space.space.clone(),
                FileSystemKey::SubSpace(sub_space) => sub_space.sub_space.space.clone(),
            }
            ResourceKey::FileSystem(filesystem) => {
                match filesystem{
                    FileSystemKey::App(app) => app.app.sub_space.space.clone(),
                    FileSystemKey::SubSpace(sub_space) => sub_space.sub_space.space.clone(),
                }
            }
        }
    }

    pub fn actor(&self)->Result<ActorKey,Fail> {
        if let ResourceKey::Actor(key) = self {
            Ok(key.clone())
        } else {
            Err(Fail::WrongResourceType)
        }
    }

    pub fn app(&self)->Result<AppKey,Fail> {
        if let ResourceKey::App(key) = self {
            Ok(key.clone())
        } else {
            Err(Fail::WrongResourceType)
        }
    }

    pub fn file(&self)->Result<FileKey,Fail> {
        if let ResourceKey::File(key) = self {
            Ok(key.clone())
        } else {
            Err(Fail::WrongResourceType)
        }
    }

    pub fn encode(&self)->Result<String,Error> {
        Ok(base64::encode(self.bin()?))
    }

    pub fn decode( string: String )->Result<Self,Error>{
        Ok(ResourceKey::from_bin(base64::decode(string)?)?)
    }



    pub fn manager(&self)->ResourceManagerKey
    {
        match self
        {
            ResourceKey::Space(_) => ResourceManagerKey::Central,
            ResourceKey::SubSpace(sub_space) => {
                //ResourceManagerKey::Key(ResourceKey::Space(sub_space.space.clone()))
                ResourceManagerKey::Central
            }
            ResourceKey::App(app) => {
                //ResourceManagerKey::Key(ResourceKey::Space(app.sub_space.space.clone()))
                ResourceManagerKey::Central
            }
            ResourceKey::Actor(actor) => {
                ResourceManagerKey::Key(ResourceKey::App(actor.app.clone()))
            }
            ResourceKey::User(user) => {
                //ResourceManagerKey::Key(ResourceKey::Space(user.space.clone()))
                ResourceManagerKey::Central
            }
            ResourceKey::File(file) => {
                //ResourceManagerKey::Key(ResourceKey::App(file.app.clone()))
                ResourceManagerKey::Central
            }
            ResourceKey::Artifact(artifact) => {
                //ResourceManagerKey::Key(ResourceKey::Space(artifact.sub_space.space.clone()))
                ResourceManagerKey::Central
            }
            ResourceKey::FileSystem(key) => {
                match key
                {
                    FileSystemKey::App(app) => {
                        //ResourceManagerKey::Key(ResourceKey::Space(app.sub_space.space.clone()))
                        ResourceManagerKey::Central
                    }
                    FileSystemKey::SubSpace(sub_space) => {
                        //ResourceManagerKey::Key(ResourceKey::Space(app.sub_space.space.clone()))
                        ResourceManagerKey::Central
                    }
                }
            }
        }

    }
}

#[derive(Clone,Serialize,Deserialize,Hash,Eq,PartialEq)]
pub enum FileSystemKey
{
    App(AppFilesystemKey),
    SubSpace(SubSpaceFilesystemKey)
}

#[derive(Clone,Serialize,Deserialize,Hash,Eq,PartialEq)]
pub struct AppFilesystemKey
{
    pub app: AppKey,
    pub id: u32
}

#[derive(Clone,Serialize,Deserialize,Hash,Eq,PartialEq)]
pub struct SubSpaceFilesystemKey
{
    pub sub_space: SubSpaceKey,
    pub id: u32
}


#[derive(Clone,Serialize,Deserialize,Hash,Eq,PartialEq)]
pub enum GatheringKey {
  Actor(ActorKey)
}

impl GatheringKey
{
    pub fn bin(&self) -> Result<Vec<u8>, Error>
    {
        let mut bin = bincode::serialize(self)?;
        Ok(bin)
    }

    pub fn from_bin(mut bin: Vec<u8>) -> Result<GatheringKey, Error>
    {
        let mut key = bincode::deserialize::<GatheringKey>(bin.as_slice())?;
        Ok(key)
    }
}

impl fmt::Display for ResourceKey{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!( f,"{}",
                match self{
                    ResourceKey::Space(key) => format!("SpaceKey:{}",key),
                    ResourceKey::SubSpace(key) => format!("SubSpaceKey:{}",key),
                    ResourceKey::App(key)  => format!("AppKey:{}",key),
                    ResourceKey::Actor(key) => format!("ActorKey:{}",key),
                    ResourceKey::User(key) => format!("UserKey:{}",key),
                    ResourceKey::File(key) => format!("FileKey:{}",key),
                    ResourceKey::Artifact(key) => format!("ArtifactKey:{}",key),
                    ResourceKey::FileSystem(key) => format!("FilesystemKey:{}", key),
                })
    }
}

impl ResourceKey
{
    pub fn resource_type(&self) -> ResourceType
    {
        match self
        {
            ResourceKey::Space(_) => ResourceType::Space,
            ResourceKey::SubSpace(_) => ResourceType::SubSpace,
            ResourceKey::App(_) => ResourceType::App,
            ResourceKey::Actor(_) => ResourceType::Actor,
            ResourceKey::User(_) => ResourceType::User,
            ResourceKey::File(_) => ResourceType::File,
            ResourceKey::Artifact(_) => ResourceType::Artifact,
            ResourceKey::FileSystem(_) => ResourceType::FileSystem
        }
    }

    pub fn sub_space(&self)->Result<SubSpaceKey,Error>
    {
        match self
        {
            ResourceKey::Space(_) => Err("space does not have a subspace".into()),
            ResourceKey::SubSpace(sub_space) => Ok(sub_space.clone()),
            ResourceKey::App(app) => Ok(app.sub_space.clone()),
            ResourceKey::Actor(actor) => Ok(actor.app.sub_space.clone()),
            ResourceKey::User(user) => Err("user does not have a sub_space".into()),
            ResourceKey::File(file) => match &file.filesystem{
                FileSystemKey::App(app) => {
                    Ok(app.app.sub_space.clone())
                }
                FileSystemKey::SubSpace(sub_space) => {
                    Ok(sub_space.sub_space.clone())
                }
            },
            ResourceKey::Artifact(artifact) => Ok(artifact.sub_space.clone()),
            ResourceKey::FileSystem(filesystem) => {
                match filesystem{
                    FileSystemKey::App(app) => {
                        Ok(app.app.sub_space.clone())
                    }
                    FileSystemKey::SubSpace(sub_space) => {
                        Ok(sub_space.sub_space.clone())
                    }
                }
            }
        }
    }


    pub fn bin(&self)->Result<Vec<u8>,Error>
    {
        let mut bin= bincode::serialize(self)?;
        bin.insert(0, self.resource_type().magic() );
        Ok(bin)
    }

    pub fn from_bin(mut bin: Vec<u8> )->Result<ResourceKey,Error>
    {
        bin.remove(0);
        let mut key = bincode::deserialize::<ResourceKey>(bin.as_slice() )?;
        Ok(key)
    }



}

impl From<Vec<Resource>> for Reply
{
    fn from(resources: Vec<Resource>) -> Self {
        Reply::Keys(resources.iter().map(|r|r.key.clone()).collect())
    }
}

impl fmt::Display for FileSystemKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!( f,"{}",
                match self{
                    FileSystemKey::App(_) => "App",
                    FileSystemKey::SubSpace(_) => "SubSpace"
                })
    }
}
