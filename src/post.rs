use std::collections::HashMap;
use worker::*;

use super::*;

pub async fn handle_post_request<S: AsRef<str>>(
    mut req: Request,
    env: &Env,
    user: Option<user_obj::User>,
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

    // Priority is login -> register -> logout
    if hashmap.contains_key("login") {
        if let Some(FormEntry::Field(email)) = form_data.get("email") {
            if let Some(FormEntry::Field(password)) = form_data.get("password") {
                let response = Response::empty();
                let mut headers = Headers::new();

                let session_id = db::create_session(env, email, password)
                    .await
                    .expect("Server failed to create session.");

                if let Some(session_id) = session_id {
                    headers
                        .set("Set-Cookie", format!("sessionId={}", session_id).as_str())
                        .unwrap();
                    headers.set("Location", req.path().as_str()).unwrap();
                    return Ok(response.unwrap().with_status(303).with_headers(headers));
                } else {
                    return Ok(render_page(&path, env, true, user).await?.with_status(200));
                }
            }
        }
        return Response::error("Bad request", 400);
    } else if hashmap.contains_key("register") {
        if let Some(FormEntry::Field(email)) = form_data.get("email") {
            if let Some(FormEntry::Field(password)) = form_data.get("password") {
                if let Some(FormEntry::Field(username)) = form_data.get("username") {
                    let response = Response::empty();
                    let mut headers = Headers::new();

                    let session_id = db::create_user(env, email, username, password).await?;

                    if let Some(session_id) = session_id {
                        headers
                            .set("Set-Cookie", format!("sessionId={}", session_id).as_str())
                            .unwrap();
                        headers.set("Location", req.path().as_str()).unwrap();
                        return Ok(response.unwrap().with_status(303).with_headers(headers));
                    } else {
                        return Ok(render_page(&path, env, true, user).await?.with_status(200));
                    }
                }
            }
        }
        return Response::error("Bad request", 400);
    }

    let user = match user {
        None => return Response::error("Error, User is not logged in!", 401),
        Some(user) => user,
    };

    if hashmap.contains_key("logout") {
        let mut headers = Headers::new();
        headers
            .set("Set-Cookie", "sessionId=deleted")
            .expect("failed to set response header");
        headers
            .set("Location", req.path().as_str())
            .expect("Failed to set response header");

        db::delete_session(
            env,
            session_id.expect("Error: User was Some but session_id was None!"),
        )
        .await?;
        let mut headers = Headers::new();
        headers.set("Location", req.path().as_str()).unwrap();
        return Ok(Response::empty()?.with_status(303).with_headers(headers));
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
            if !title.contains(validchar) {
                return Response::error("Char must be alphanumeric", 400);
            }

            // Ensure path exists
            if db::get_content(env, post_id).await?.is_none() {
                return Response::error("Error: Can only reply to a post that exists", 400);
            }
            // Ensure fulltitle doesn't exist
            if db::get_content(env, fulltitle.as_str()).await?.is_some() {
                return Response::error("Error: post already exists", 409);
            }
            // Ensure total length is <= 512
            if fulltitle.len() >= 512 {
                return Response::error("Error: max length has been reached", 400);
            }

            // actually save new post content
            db::post_content(env, fulltitle.as_str(), content.as_str(), user).await?;

            // create reponse to redirect user to new page
            let response = Response::empty()?;
            let mut headers = Headers::new();
            headers.set("Location", format!("/{}", fulltitle).as_str())?;

            return Ok(response.with_status(303).with_headers(headers));
        }
    }
    Response::error("Bad request, title and content must both be present.", 400)
}

fn validchar(c: char) -> bool {
    ('0'..='9').contains(&c) || ('a'..='z').contains(&c) || ('A'..='Z').contains(&c)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn valid_char() {
        assert!(validchar('z'), "z is valid char");
        assert!(validchar('0'), "4 is valid char");
        assert!(validchar('A'), "A is valid char");
        assert!(!validchar('$'), "$ is not valid char");
        assert!(!validchar('!'), "! is not valid char");
        assert!(!validchar('#'), "# is not valid char");
        assert!(!validchar('~'), "~ is not valid char");
    }
}
