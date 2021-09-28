use worker::*;
mod db;
mod utils;

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or("unknown region".into())
    );
}

#[event(fetch)]
pub async fn main(mut req: Request, env: Env) -> Result<Response> {
    log_request(&req);
    utils::set_panic_hook();

    match req.method() {
        Method::Get => {
            let style = include_str!("html/index.css");

            // Get post id from path
            let path = req.path();
            let post_id = path.strip_prefix("/").unwrap(); //path always starts with /

            // get content, return error if page doesn't exists
            let content = match db::get_content(&env, post_id).await? {
                None => {
                    return Response::error("Page Not Found", 404);
                }
                Some(content) => content,
            };

            // get all replies to post
            let replies = db::get_replies(&env, post_id).await?;

            // Render replies
            let replies_html = replies
                .iter()
                .map(|(id, content)| {
                    include_str!("html/templates/post.html")
                        .replace("<!--title-->", id)
                        .replace("<!--content-->", content)
                })
                .collect::<String>();
            // render page
            let response = include_str!("html/index.html")
                .replace("/*style*/", style)
                .replace("<!--title-->", post_id)
                .replace("<!--content-->", content.as_str())
                .replace("<!--replies-->", replies_html.as_str());

            Response::from_html(response)
        }
        Method::Post => {
            // Get post_id from path
            let path = req.path();
            let post_id = path.strip_prefix("/").unwrap(); //path always starts with /

            // Check if login/register param is present; if so, process login/register input
            let url = req.url()?;
            let pairs = url.query_pairs();

            // get form data
            let form_data = req.form_data().await?;

            let mut resp: Option<Result<Response>> = None;

            // The second parameter in the url will always take precedent, so ?login&register will result in a register request
            pairs.for_each(|kv| {
                if kv.0 == "login" {
                    if let Some(FormEntry::Field(email)) = form_data.get("email") {
                        if let Some(FormEntry::Field(password)) = form_data.get("password") {
                            let response = Response::empty();
                            let mut headers = Headers::new();

                            headers
                                .set("Set-Cookie", "sessionId=DUMMY_SESSION_ID")
                                .unwrap();
                            headers.set("Location", req.path().as_str()).unwrap();
                            resp =
                                Some(Ok(response.unwrap().with_status(303).with_headers(headers)));
                            return;
                        }
                    }
                    resp = Some(Response::error("Bad request", 400));
                } else if kv.0 == "register" {
                    // TODO: parse form_data as register request and set resp
                    resp = Some(Response::error(
                        "Come back soon! Development is BEaSt MOdE",
                        501,
                    ));
                }
            });

            if let Some(resp) = resp {
                return resp;
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
                    // Ensure path exists
                    if let None = db::get_content(&env, post_id).await? {
                        return Response::error("Error: Can only reply to a post that exists", 400);
                    }
                    // Ensure fulltitle doesn't exist
                    if let Some(_) = db::get_content(&env, fulltitle.as_str()).await? {
                        return Response::error("Error: post already exists", 409);
                    }
                    // Ensure total length is <= 512
                    if fulltitle.len() >= 512 {
                        return Response::error("Error: max length has been reached", 400);
                    }

                    // actually save new post content
                    db::post_content(&env, fulltitle.as_str(), content.as_str()).await?;

                    // create reponse to redirect user to new page
                    let response = Response::empty();
                    let mut headers = Headers::new();
                    headers.set("Location", format!("/{}", fulltitle).as_str())?;
                    return Ok(response?.with_status(303).with_headers(headers));
                }
            }
            Response::error("Bad request, title and content must both be present.", 400)
        }
        Method::Put => {
            todo!()
        }

        _ => Response::error("Only GET, PUT and POST methods are allowed", 405),
    }
}
