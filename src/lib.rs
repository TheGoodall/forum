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

    let index = include_str!("html/index.html");
    let style = include_str!("html/index.css");

    Response::from_html(str::replace(index, "/*style*/", style))
}

async fn get_content(env: Env, post_id: String) -> Result<Option<String>> {
    let prefix = get_prefix(post_id.as_str());
    let data = env.kv("POSTS")?.get(prefix.as_str()).await?;
    let content: Option<String>;
    if let Some(contents) = data {
        content = Some(contents.as_string());
    } else {
        content = None;
    }
    
    Ok(content)

}

async fn post_content(env: Env, post_id: &str, contents: &str) -> Result<()> {
    let kv = env.kv("POSTS")?;
    let prefix = get_prefix(post_id);
    kv.put(prefix.as_str(), contents)?.execute().await?;
    Ok(())

}

async fn get_replies(env: Env, post_id: &str) -> Result<Vec<(String, String)>> {
    let limit = 50;
    let prefix = get_prefix(post_id);

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
        async move { (key_name.trim_start().to_string(), body.await.unwrap().unwrap().as_string()) }
    });
    let test = join_all(replies).await;
    Ok(test)
}

fn get_prefix(post_id: &str) -> String {
    let key_length = post_id.len();
    let zeros = " "
        .chars()
        .cycle()
        .take(512 - key_length)
        .collect::<String>();
    format!("{}{}", zeros, post_id)
}
