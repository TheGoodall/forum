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
pub async fn main(req: Request, env: Env) -> Result<Response> {
    log_request(&req);
    utils::set_panic_hook();

    match req.method() {
        Method::Get => {
            let style = include_str!("html/index.css");
            let path = req.path();
            let post_id = path.strip_prefix("/").unwrap(); //path always starts with /
            let content = match get_content(&env, post_id).await? {
                None => {
                    return Response::error("Page Not Found", 404);
                }
                Some(content) => content,
            };
            let replies = get_replies(&env, post_id).await?;
            let replies_html = replies
                .iter()
                .map(|(id, content)| {
                    include_str!("html/templates/post.html")
                        .replace("<!--title-->", id)
                        .replace("<!--content-->", content)
                })
                .collect::<String>();

            let response = include_str!("html/index.html")
                .replace("/*style*/", style)
                .replace("<!--title-->", post_id)
                .replace("<!--content-->", content.as_str())
                .replace("<!--replies-->", replies_html.as_str());

            Response::from_html(response)
        }
        Method::Post => Response::empty(),
        _ => Response::error("Only Get and Post methods are allowed", 405),
    }
}

async fn get_content(env: &Env, post_id: &str) -> Result<Option<String>> {
    let prefix = get_prefix(post_id, 0);
    let data = env.kv("POSTS")?.get(prefix.as_str()).await?;
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

    let keys = env
        .kv("POSTS")?
        .list()
        .prefix(prefix)
        .limit(limit)
        .execute()
        .await?;
    let kv = env.kv("POSTS")?;
    let replies = keys.keys.iter().map(|key| {
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

fn get_prefix(post_id: &str, offset: usize) -> String {
    let key_length = post_id.len();
    let zeros = " "
        .chars()
        .cycle()
        .take((512 - key_length)-offset)
        .collect::<String>();
    format!("{}{}", zeros, post_id)
}
