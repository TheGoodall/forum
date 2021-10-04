use std::collections::HashMap;

use worker::*;
mod crypto_helpers;
mod db;
mod post;
mod render;
mod user_obj;
mod utils;
use post::handle_post_request;
use render::render_page;

#[event(fetch)]
pub async fn main(req: Request, env: Env) -> Result<Response> {
    utils::log_request(&req);
    utils::set_panic_hook();

    let session_id = match req.headers().get("Cookie")? {
        None => {
            return Ok(Response::empty().unwrap().with_status(400));
        }
        Some(cookies) => {
            let map: HashMap<_, _> = cookies
                .split(';')
                .map(|cookie| {
                    let kvp = cookie
                        .split('=')
                        .take(2)
                        .map(|text| text.trim())
                        .collect::<Vec<&str>>();
                    (kvp[0].to_owned(), kvp[1].to_owned())
                })
                .collect();
            match map.get("sessionId") {
                None => {
                    return Response::error("Not authorised", 401);
                }
                Some(session_id) => session_id.to_owned(),
            }
        }
    };

    match req.method() {
        Method::Get => render_page(&req.path(), env, false).await,
        Method::Post => handle_post_request(req, env, &session_id).await,
        Method::Put => {
            todo!()
        }

        _ => Response::error("Only GET, PUT and POST methods are allowed", 405),
    }
}
