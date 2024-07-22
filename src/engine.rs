use crate::context::Context;
use crate::proto::{
    filler_server::{Filler, FillerServer},
    Ad, Request, Response,
};
use crate::AsyncResult;

use tonic::{metadata::Ascii, metadata::MetadataValue, transport::Server};

use std::net::SocketAddr;
use std::time::Duration;

#[derive(Debug)]
pub(crate) struct FillerService {
    context: Context,
}

impl FillerService {
    pub fn new(context: Context) -> Self {
        Self { context }
    }
}

#[tonic::async_trait]
impl Filler for FillerService {
    async fn fill(
        &self,
        request: tonic::Request<Request>,
    ) -> Result<tonic::Response<Response>, tonic::Status> {
        let input = request.get_ref();
        let mut response = Response::default();
        let mut mask = self.context.inventory.load().filters.all_pass.to_owned();

        // Category restriction
        for cat in input.categories.iter() {
            if let Some(x) = &self.context.inventory.load().filters.category.get(cat) {
                mask = mask.and(x);
                // Nothing available for this combination
                if mask.is_empty() {
                    return Ok(tonic::Response::new(response));
                }
            }
        }

        // Hour restriction
        let candidates =
            &self.context.inventory.load().filters.hour[self.context.cached_time.hour()];
        mask = mask.and(candidates);

        // Collect ads
        let mut ads = vec![];
        for value in mask.iter() {
            let index = value as usize;
            let ad = &self.context.inventory.load().ads[index];
            if ad.id.is_empty() {
                continue;
            }
            ads.push(Ad { id: ad.id.clone() });
        }

        response.ads = ads;
        Ok(tonic::Response::new(response))
    }
}

pub(crate) async fn run(context: Context) -> AsyncResult {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000)); // TODO: From config

    // Reflection
    let service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(tonic_reflection::pb::FILE_DESCRIPTOR_SET)
        .build()?;

    // gRPC server
    let filler = FillerService::new(context);
    Server::builder()
        .timeout(Duration::from_secs(10))
        .tcp_keepalive(Some(Duration::from_secs(30)))
        .add_service(service)
        .add_service(FillerServer::with_interceptor(filler, check_auth))
        .serve(addr)
        .await?;

    Ok(())
}

lazy_static::lazy_static! {
    static ref TOKEN: MetadataValue<Ascii> = "Bearer my-secret-token".parse().unwrap();
}

fn check_auth(req: tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status> {
    match req.metadata().get("authorization") {
        Some(t) if *TOKEN == t => Ok(req),
        _ => Err(tonic::Status::unauthenticated("No valid auth token")),
    }
}
