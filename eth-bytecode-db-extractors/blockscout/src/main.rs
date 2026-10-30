// SPDX-License-Identifier: LicenseRef-Blockscout

use anyhow::Context;
use blockscout::{Client, Settings};
use blockscout_service_launcher::{self as launcher, launcher::ConfigSettings};
use migration::Migrator;

const SERVICE_NAME: &str = "blockscout-extractor";

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    launcher::tracing::init_logs(SERVICE_NAME, &Default::default(), &Default::default())
        .context("tracing initialization")?;

    let settings = Settings::build().context("failed to read config")?;

    let database_settings = launcher::database::DatabaseSettings {
        connect: launcher::database::DatabaseConnectSettings::Url(settings.database_url.clone()),
        create_database: settings.create_database,
        run_migrations: settings.run_migrations,
        connect_options: Default::default(),
    };
    let db_connection =
        launcher::database::initialize_postgres::<Migrator>(&database_settings).await?;

    let client = Client::try_new(
        db_connection,
        settings.chain_id,
        settings.blockscout_url,
        settings.eth_bytecode_db_url,
        settings.eth_bytecode_db_api_key,
        settings.limit_requests_per_second,
    )?;

    tracing::info!("importing contract addresses started");
    let processed = client
        .import_contract_addresses(settings.force_import)
        .await?;
    tracing::info!("importing contract addresses finished. Total processed contracts={processed}");

    let mut handles = Vec::with_capacity(settings.n_threads);
    for _ in 0..settings.n_threads {
        let client = client.clone();
        let handle = tokio::spawn(client.verify_contracts());
        handles.push(handle);
    }
    for result in futures::future::join_all(handles).await {
        result.context("join handle")?.context("verify contracts")?;
    }

    Ok(())
}
