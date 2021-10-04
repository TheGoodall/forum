use std::collections::HashMap;
use worker::*;

use super::*;

pub async fn handle_post_request<S: AsRef<str>>(
    mut req: Request,
    env: Env,
    session_id: Option<S>,
) -> Result<Response> {
    let session_id = session_id.as_ref();
    // Get post_id from path
    let path = req.path();
    let post_id = path
        .strip_prefix('/')
        .expect("Expected path to begin with /");

    // Check if login/register param is present; if so, process login/register input
    let url = req.url()?;
    let pairs = url.query_pairs();
    let hashmap: HashMap<_, _> = pairs.to_owned().collect();

    // get form data
    let form_data = req.form_data().await?;

    // The second parameter in the url will always take precedent, so ?login&register will result in a register request
    if hashmap.contains_key("login") {
        if let Some(FormEntry::Field(email)) = form_data.get("email") {
            if let Some(FormEntry::Field(password)) = form_data.get("password") {
                let response = Response::empty();
                let mut headers = Headers::new();

                let session_id = db::create_session(&env, email, password)
                    .await
                    .expect("Server failed to create session.");

                if let Some(session_id) = session_id {
                    headers
                        .set("Set-Cookie", format!("sessionId={}", session_id).as_str())
                        .unwrap();
                    headers.set("Location", req.path().as_str()).unwrap();
                    return Ok(response.unwrap().with_status(303).with_headers(headers));
                } else {
                    return Ok(render_page(&path, env, true).await?.with_status(200));
                }
            }
        }
        return Response::error("Bad request", 400);
    } else if hashmap.contains_key("register") {
        if let Some(FormEntry::Field(email)) = form_data.get("email") {
            if let Some(FormEntry::Field(password)) = form_data.get("password") {
                let response = Response::empty();
                let mut headers = Headers::new();

                let session_id = db::create_user(env, email, password)
                    .await
                    .expect("Server failed to create user.");

                if let Some(session_id) = session_id {
                    headers
                        .set("Set-Cookie", format!("sessionId={}", session_id).as_str())
                        .unwrap();
                    headers.set("Location", req.path().as_str()).unwrap();
                    return Ok(response.unwrap().with_status(303).with_headers(headers));
                } else {
                    return Ok(response.unwrap().with_status(400));
                }
            }
        }
        return Response::error("Bad request", 400);
    }

    let session_id = if let Some(session_id) = session_id {
        session_id
    } else {
        return Response::error("Error, User is not logged in!", 401);
    };

    if hashmap.contains_key("logout") {
        let mut headers = Headers::new();
        headers
            .set("Set-Cookie", "sessionId=deleted")
            .expect("failed to set response header");
        headers
            .set("Location", req.path().as_str())
            .expect("Failed to set response header");

        db::delete_session(&env, session_id).await?;
    }

    // unpack form data and ensure that the correct attributes exist.
    if let Some(FormEntry::Field(title)) = form_data.get("title") {
        if let Some(FormEntry::Field(content)) = form_data.get("content") {
            // Assemble full title from old title and new char
            let fulltitle = format!("{}{}", post_id, title);

            // Ensure title is one char
            if title.len() != 1 {
                return Response::error("Error: Only one char can be added at a time", 400);
            }

            // Ensure Ensure title is a valid char
            let validchar = |c: char| {
                ('0'..='9').contains(&c) || ('a'..='z').contains(&c) || ('A'..='Z').contains(&c)
            };
            if title.contains(validchar) {
                return Response::error("Char must be alphanumeric", 400);
            }

            // Ensure path exists
            if db::get_content(&env, post_id).await?.is_none() {
                return Response::error("Error: Can only reply to a post that exists", 400);
            }
            // Ensure fulltitle doesn't exist
            if db::get_content(&env, fulltitle.as_str()).await?.is_some() {
                return Response::error("Error: post already exists", 409);
            }
            // Ensure total length is <= 512
            if fulltitle.len() >= 512 {
                return Response::error("Error: max length has been reached", 400);
            }

            let response = match db::get_session(&env, session_id).await? {
                None => Response::error("Not authorised", 401),
                Some(_) => {
                    // actually save new post content
                    db::post_content(&env, fulltitle.as_str(), content.as_str()).await?;
                    let response = Response::empty();
                    let mut headers = Headers::new();
                    headers.set("Location", format!("/{}", fulltitle).as_str())?;
                    Ok(response?.with_status(303).with_headers(headers))
                }
            };
            return response;

            // create reponse to redirect user to new page
        }
    }
    Response::error("Bad request, title and content must both be present.", 400)
}
