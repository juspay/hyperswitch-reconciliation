use actix_web::{web, Scope};
use recon_interfaces::secrets_interface::secret_state::RawSecret;
use storage_impl::Store;

use super::{api_docs, health};
use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub config: Config<RawSecret>,
    /// Master database, shared with recon.
    pub store: Store,
}

pub struct Health;

impl Health {
    pub fn server(state: AppState) -> Scope {
        web::scope("/health")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::get().to(health::get_health)))
            .service(web::resource("/deep").route(web::get().to(health::get_deep_health)))
    }
}

pub struct ApiDocs;

impl ApiDocs {
    pub fn server(state: AppState) -> Scope {
        web::scope("/api_docs")
            .app_data(web::Data::new(state))
            .service(
                web::resource("/openapi.json").route(web::get().to(api_docs::get_openapi_spec)),
            )
    }
}
