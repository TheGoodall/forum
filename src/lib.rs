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
    let kv = env.kv("POSTS")?;
    let post_name = req.path().as_str()[1..];
    let keys = kv.list().prefix("".to_string()).execute().await?.keys;
    let values = keys.iter()
        .map(|key| {
            kv.get(key.name.as_str())
        })
        .collect::<Vec<_>>();
    console_log!("{:#?}", join_all(values).await);
    Response::from_html(str::replace(index, "{{post_name}}", &post_name))
}
