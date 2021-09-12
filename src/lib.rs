use worker::*;
use futures::future::join_all;
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
    let stylesheet = include_str!("html/stylesheet.css");
    Response::from_html(index)
}

async fn get_post_content(env: Env, postid: String) -> Result<String> {
    todo!()

}

async fn get_replies(env: Env, postid: String) -> Result<Vec<(String,String)>> {
    let kv = env.kv("POSTS")?;
    let keys = kv.list().execute().await?.keys;
    let values = keys.iter()
        .map(|key| {
            kv.get(key.name.as_str())
        })
        .collect::<Vec<_>>();
    let posts = join_all(values).await;
    console_log!("{:#?}", posts);
    todo!()
}
