use std::str::FromStr;
use mesh_portal::version::latest::entity::request::create::{PointTemplate, KindTemplate, Template, PointSegFactory};
use mesh_portal::version::latest::id::{Point, RouteSegment};
use mesh_portal::version::latest::messaging::{Message, Request};
use tokio::sync::mpsc;
use crate::message::delivery::Delivery;
use crate::star::StarKey;

lazy_static! {
    pub static ref HYPERUSER: &'static str ="hyperspace:users:hyperuser";
    pub static ref HYPER_USERBASE: &'static str ="hyperspace:users";
}

pub struct HyperUser {

}

impl HyperUser {
    pub fn address() -> Point {
        Point::from_str(&HYPERUSER).expect("should be a valid hyperuser address")
    }

    /*
    pub fn template() -> Template {
        Template {
            point:
            PointTemplate { parent: Point::root_with_route(RouteSegment::Mesh(StarKey::central().to_string())), child_segment_template: PointSegFactory::Exact("hyperuser".to_string()) },
            kind: KindTemplate {
                kind: "User".to_string(),
                sub_kind: None,
                specific: None
            }
        }
    }

     */

    pub fn messenger() -> mpsc::Sender<Message> {
        let (messenger_tx,mut messenger_rx) = mpsc::channel(1024);
        tokio::spawn( async move {
            // right now we basically ignore messages to HyperUser
            while let Option::Some(_) = messenger_rx.recv().await {}
        });
        messenger_tx
    }
}