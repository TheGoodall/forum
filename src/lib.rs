use futures::future::join_all;
use worker::*;
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
            let content = match get_content(&env, post_id).await? {
                None => {
                    return Response::error("Page Not Found", 404);
                }
                Some(content) => content,
            };

            // get all replies to post
            let replies = get_replies(&env, post_id).await?;

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
        Method::Put => {
            // Get post_id from path
            let path = req.path();
            let post_id = path.strip_prefix("/").unwrap(); //path always starts with /

            // get form data
            let form_data = req.form_data().await?;

            // unpack form data and ensure that the correct attributes exist.
            if let Some(FormEntry::Field(title)) = form_data.get("title") {
                if let Some(FormEntry::Field(content)) = form_data.get("content") {
                    // Assemble full title from old title and new char
                    let fulltitle = format!("{}{}", post_id, title);

                    // Ensure title is one char
                    // Ensure Ensure title is a valid char
                    // Ensure path exists
                    // Ensure fulltitle doesn't exist
                    if let Some(_) = get_content(&env, fulltitle.as_str()).await? {
                        return Response::error("Error: post already exists", 409);
                    }
                    // Ensure total length is <= 512

                    // actually save new post content
                    post_content(&env, fulltitle.as_str(), content.as_str()).await?;

                    // create reponse to redirect user to new page
                    let response = Response::empty();
                    let mut headers = Headers::new();
                    headers.set("Location", format!("/{}", fulltitle).as_str())?;
                    return Ok(response?.with_status(303).with_headers(headers));
                }
            }
            Response::error("Bad request, title and content must both be present.", 400)
        }

        _ => Response::error("Only GET and POST methods are allowed", 405),
    }
}

async fn get_content(env: &Env, post_id: &str) -> Result<Option<String>> {
    let prefix = get_prefix(post_id, 0);

    // get data
    let data = env.kv("POSTS")?.get(prefix.as_str()).await?;

    // convert to string and return
    let content: Option<String>;
    if let Some(contents) = data {
        content = Some(contents.as_string());
    } else {
        content = None;
    }

    Ok(content)
}

async fn post_content(env: &Env, post_id: &str, contents: &str) -> Result<()> {
    let kv = env.kv("POSTS")?;
    let prefix = get_prefix(post_id, 0);
    kv.put(prefix.as_str(), contents)?.execute().await?;
    Ok(())
}

async fn get_replies(env: &Env, post_id: &str) -> Result<Vec<(String, String)>> {
    let limit = 50;
    let prefix = get_prefix(post_id, 1);

    // get list of keys with correct prefix
    let keys = env
        .kv("POSTS")?
        .list()
        .prefix(prefix)
        .limit(limit)
        .execute()
        .await?;
    let kv = env.kv("POSTS")?;

    // get content for each key
    let replies = keys
        .keys
        .iter()
        .filter(|key| {
            if let Some(_) = key.name.rfind(|c: char| !c.is_whitespace()) {
                true
            } else {
                false
            }
        })
        .map(|key| {
            let key_name = key.name.as_str();
            let body = kv.get(key_name);
            async move {
                (
                    key_name.trim_start().to_string(),
                    body.await.unwrap().unwrap().as_string(),
                )
            }
        });
    let test = join_all(replies).await;
    Ok(test)
}

/*
 *  add zeros to prefix to ensure it is in the correct format e.g. right-justified
 */
fn get_prefix(post_id: &str, offset: usize) -> String {
    let key_length = post_id.len();
    let zeros = " "
        .chars()
        .cycle()
        .take((512 - key_length) - offset)
        .collect::<String>();
    format!("{}{}", zeros, post_id)
}
