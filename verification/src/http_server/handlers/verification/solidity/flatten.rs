use super::types::{FlattenedSource, VerificationRequest};
use crate::{
    download_cache::DownloadCache, http_server::handlers::verification::VerificationResponse,
    solidity::fetcher::SvmFetcher,
};
use actix_web::{
    error,
    web::{self, Json},
    Error,
};
use ethers_solc::CompilerInput;

pub async fn verify(
    cache: web::Data<DownloadCache<SvmFetcher>>,
    params: Json<VerificationRequest<FlattenedSource>>,
) -> Result<Json<VerificationResponse>, Error> {
    let params = params.into_inner();

    let input = CompilerInput::try_from(params.content).map_err(error::ErrorBadRequest)?;
    let output = super::compile::compile(&cache, &params.compiler_version, &input).await?;
    // TODO: verify output
    let _ = output;

    todo!()
}
