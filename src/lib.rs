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

    // Get session_id
    let mut session_id = req
        .headers()
        .get("Cookie")?
        .map(|cookies| {
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
            map.get("sessionId").map(|session_id| session_id.to_owned())
        })
        .flatten();

    // Get user for session if valid session else None
    let user = if let Some(ref session_id) = session_id {
        db::get_session(&env, &session_id).await?
    } else {
        None
    };

    // remove session_ids that do not correspond to a valid session
    session_id = session_id.filter(|_| user.is_some());

    let result = match req.method() {
        Method::Get => render_page(&req.path(), &env, false, user).await,
        Method::Post => handle_post_request(req, &env, user, session_id).await,
        _ => Response::error("Only GET and POST methods are allowed", 405),
    };

    // If the route returns an error, replace it with an error response and return it to the user.
    match result {
        Ok(response) => Ok(response),
        Err(error) => {
            console_log!("An error occured: {}", error);
            Response::error("An error has occured", 500)
        }
    }
}
