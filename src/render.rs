use worker::*;
use super::db;

pub async fn render_page(path: &str, env: Env, is_login_error: bool) -> Result<Response> {
    let style = include_str!("html/index.css");

    // Get post id from path
    let post_id = path
        .strip_prefix('/')
        .expect("Expected path to begin with /");

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

    let html = match is_login_error {
        true => response.replace("<!--loginError-->", "Invalid Username or password"),
        false => response.replace("<!--loginError-->", ""),
    };
    Response::from_html(html)
}
