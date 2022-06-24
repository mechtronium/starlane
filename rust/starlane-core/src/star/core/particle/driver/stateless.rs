use mesh_portal::version::latest::command::common::StateSrc;
use mesh_portal::version::latest::id::Point;
use mesh_portal_versions::version::v0_0_1::id::id::KindBase;
use mesh_portal_versions::version::v0_0_1::sys::Assign;

use crate::error::Error;
use crate::star::core::particle::driver::ParticleCoreDriver;
use crate::star::core::particle::state::StateStore;
use crate::star::StarSkel;

#[derive(Debug)]
pub struct StatelessCoreDriver {
    skel: StarSkel,
    resource_type: KindBase
}

impl StatelessCoreDriver {
    pub async fn new(skel: StarSkel, resource_type: KindBase) -> Self {
        StatelessCoreDriver {
            skel: skel.clone(),
            resource_type
        }
    }
}

#[async_trait]
impl ParticleCoreDriver for StatelessCoreDriver {

    fn kind(&self) -> KindBase {
        self.resource_type.clone()
    }


    async fn assign(
        &mut self,
        assign: Assign,
    ) -> Result<(), Error> {
        match assign.state {
            StateSrc::None=> {
            }
            StateSrc::Substance(_) => {
                return Err("must be stateless".into());
            }
        };
        Ok(())
    }


}
