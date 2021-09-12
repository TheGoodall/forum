use serde_json::json;
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


    let router = Router::new(());
    router
        .get_async("/", |_req, ctx| async move {
            let index = include_str!("html/index.html");
            let kv = ctx.kv("POSTS")?;
            let keys = kv.list().execute().await?.keys;
            let values = keys.iter()
                .map(|key| async {
                    kv.get(key.name.as_str())
                })
                .collect::<Vec<_>>();
            console_log!("{:#?}", );
            Response::from_html(index)
    })
        .run(req, env).await
}
