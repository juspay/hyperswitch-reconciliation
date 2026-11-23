use recon_env::logging;
use reporting::{
    config::{ApplicationResult, CmdLineConf, Config},
    errors::ReportingAppError,
    initialize_server,
};

#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[tokio::main]
async fn main() -> ApplicationResult<()> {
    let cmd_line = <CmdLineConf as clap::Parser>::parse();

    #[allow(clippy::expect_used)]
    let conf = Config::with_config_path(cmd_line.config_path.clone())
        .expect("Unable to construct reporting service configuration");

    #[allow(clippy::expect_used)]
    conf.validate()
        .expect("Failed to validate reporting service configuration");

    #[allow(clippy::print_stdout)] // The logger has not yet been initialized
    {
        println!("Starting reporting service: {}", recon_env::env::which());
    }

    logging::init(conf.logging.clone());
    logging::info!(
        "Application started [{:?}] [{:?}]",
        conf.server,
        conf.logging
    );

    match initialize_server(conf).await {
        Ok((server, port, _config)) => {
            logging::info!("Server listening on port {}", port);

            if let Err(e) = server.await {
                logging::error!("Server error: {}", e);
            }
        }
        Err(e) => {
            logging::error!("Failed to initialize server: {e}");
            return Err(ReportingAppError::InternalServerError.into());
        }
    }

    Err(ReportingAppError::InternalServerError.into())
}
