mod db;
mod nats_api;
mod proto {
    tonic::include_proto!("chronolens");
}

use db::DbAccess;
use jsonwebtoken::{EncodingKey, Header};
use proto::{
    chrono_lens_server::{ChronoLens, ChronoLensServer},
    LoginRequest, LoginResponse,
};
use serde::Deserialize;
use tonic::{async_trait, transport::Server, Request, Response, Status};

#[derive(Deserialize, Debug)]
struct EnvVars {
    #[serde(alias = "LISTEN_ON")]
    #[serde(default = "listen_on_default")]
    listen_on: String,
    #[serde(alias = "JWT_SECRET")]
    jwt_secret: String,
}
fn listen_on_default() -> String {
    "0.0.0.0:8080".to_string()
}

struct ChronoLensService {
    database: DbAccess,
    secret: EncodingKey,
}

#[async_trait]
impl ChronoLens for ChronoLensService {
    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        let login_request = request.into_inner();

        let password_hash = match self.database.get_user_password(login_request.username).await {
            Ok(pw) => pw.password,
            Err(..) => return Err(Status::not_found("Invalid username or password")),
        };

        let matched = match bcrypt::verify(login_request.password, &password_hash) {
            Ok(matched) => matched,
            Err(..) => return Err(Status::not_found("Invalid username or password")),
        };

        if matched {
            #[derive(serde::Serialize)]
            struct Claims {
                iat: i64,
                nbf: i64,
            }
            let claims = Claims {
                iat: chrono::offset::Local::now().timestamp_millis(),
                nbf: chrono::offset::Local::now().timestamp_millis() + 604_800_000,
            };

            let token = match jsonwebtoken::encode(&Header::default(), &claims, &self.secret) {
                Ok(token) => token,
                Err(..) => panic!("Error generating JWT token"),
            };
            Ok(Response::new(LoginResponse { token }))
        } else {
            Err(Status::not_found("Invalid username or password"))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let environment_variables = match envy::from_env::<EnvVars>() {
        Ok(vars) => vars,
        Err(err) => panic!("{}", err),
    };

    let database = match DbAccess::new().await {
        Ok(database) => database,
        Err(err) => panic!("{}", err),
    };

    let addr = environment_variables.listen_on.parse()?;

    let secret = EncodingKey::from_secret(environment_variables.jwt_secret.as_ref());
    let chronolens = ChronoLensService { database, secret };

    Server::builder()
        .add_service(ChronoLensServer::new(chronolens))
        .serve(addr)
        .await?;

    Ok(())
}
