#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
extern crate rocket_contrib;
extern crate serde_json;

mod auth_actor;
use rocket::fs::FileServer;
use rocket::config::Config;
use rocket::config::TlsConfig;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::path::Path;
use rocket::fs::relative;
use rocket::State;

pub const CHALLENGE_SIZE_BYTES: usize = 32;

use std::io::Cursor;
use std::path;

use rocket::serde::json::Json;
use rocket::{response::Responder, Response};
use webauthn_rs::{
    ephemeral::WebauthnEphemeralConfig,
    error::WebauthnError,
    proto::{
        CreationChallengeResponse, PublicKeyCredential, RegisterPublicKeyCredential,
        RequestChallengeResponse,
    },
};

use crate::auth_actor::WebauthnActor;

#[get("/")]
fn index() -> &'static str {
    "Hello, world 123"
}

struct WrappedWebauthnError(WebauthnError);

impl<'r, 'o: 'r> Responder<'r, 'o> for WrappedWebauthnError {
    fn respond_to(self, _request: &'r rocket::Request<'_>) -> rocket::response::Result<'o> {
        let res = format!("{:?}", self.0);
        println!("Suck my dick fool: {:?}", res);
        Response::build()
            .sized_body(res.len(), Cursor::new(res))
            .ok()
    }
}

#[post("/auth/challenge/register/<username>")]
async fn register_challenge(
    auth: &State<WebauthnActor>,
    username: &str
) -> Result<Json<CreationChallengeResponse>, WrappedWebauthnError> {
    let actor_res = auth.challenge_register(username.to_string()).await;

    match actor_res {
        Ok(res) => Ok(Json(res)),
        Err(e) => Err(WrappedWebauthnError(e)),
    }
}

#[post("/auth/register/<username>", data = "<credential>")]
async fn register(
    username: &str,
    credential: Json<RegisterPublicKeyCredential>,
    auth: &State<WebauthnActor>,
) -> Result<(), WrappedWebauthnError> {
    let actor_res = auth.register(&username.to_string(), &credential).await;
    match actor_res {
        Ok(()) => Ok(()),
        Err(e) => Err(WrappedWebauthnError(e)),
    }
}

#[post("/auth/challenge/login/<username>")]
async fn login_challenge(
    username: &str,
    auth: &State<WebauthnActor>,
) -> Result<Json<RequestChallengeResponse>, WrappedWebauthnError> {
    let actor_res = auth.challenge_authenticate(&username.to_string()).await;

    debug!("{:?}", actor_res);

    match actor_res {
        Ok(chal) => Ok(Json(chal)),
        Err(e) => Err(WrappedWebauthnError(e)),
    }
}

#[post("/auth/login/<username>", data = "<credential>")]
async fn login(
    username: &str,
    credential: Json<PublicKeyCredential>,
    auth: &State<WebauthnActor>,
) -> Result<(), WrappedWebauthnError> {
    let username_copy = username.to_string();

    match auth.authenticate(&username_copy, &credential).await {
        Ok(()) => Ok(()),
        Err(e) => Err(WrappedWebauthnError(e)),
    }

    //@TODO: update session with creds (or give token back or some shit)
    // let session = request.session_mut();

    // Clear the anonymous flag
    // session.remove("anonymous");

    // Set the userid
    // session.insert_raw("userid", username);
}

#[launch]
fn rocket() -> _ {
    let auth_actor = WebauthnActor::new(WebauthnEphemeralConfig::new(
        "devish.com",
        "https://devish.com",
        "devish.com",
        None,
    ));

    let config = Config::default();

    let cert_path = relative!("server.crt");
    let key_path = relative!("server.key");

    let mut rocket_config = Config {
        tls: Some(TlsConfig::from_paths(cert_path, key_path)),
        ..Default::default()
    };

    // rocket_config.port = 80;
    rocket_config.address = Ipv4Addr::new(127, 0, 0, 1).into();
    rocket_config.port = 443;
    
    println!("THE FUCK BRO: {:?}", rocket_config);

    rocket::build()
        .manage(auth_actor)
        .configure(rocket_config)
        .mount(
            "/",
            routes![index, register_challenge, login_challenge, register, login],
        )
        .mount("/public", FileServer::from("clients/web/dist"))
        
}
