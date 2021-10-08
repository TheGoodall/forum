use super::db;
use super::user_obj::*;
use regex::Regex;
use worker::*;

pub async fn render_page(
    path: &str,
    env: &Env,
    is_login_error: bool,
    user: Option<User>,
) -> Result<Response> {
    let style = include_str!("html/index.css");

    // Get post id from path
    let post_id = path
        .strip_prefix('/')
        .expect("Expected path to begin with /");

    // get content, return error if page doesn't exists
    let content = match db::get_content(env, post_id).await? {
        None => {
            return Response::error("Page Not Found", 404);
        }
        Some(content) => content,
    };

    // get all replies to post
    let replies = db::get_replies(env, post_id).await?;

    // Render replies
    let replies_html = replies
        .iter()
        .map(|post| {
            let reply = include_str!("html/templates/post.html")
                .replace("<!--title-->", post.title.as_str())
                .replace("<!--content-->", post.post.content.as_str());
            let user_text = match &post.user {
                None => "[DELETED]",
                Some(user) => user.account.username.as_str(),
            };
            reply.replace("<!--user-->", user_text)
        })
        .collect::<String>();
    // render page

    let username = match content.user {
        Some(user) => user.account.username,
        None => "[Deleted]".to_owned(),
    };

    let mut response = include_str!("html/index.html")
        .replace("/*style*/", style)
        .replace("<!--title-->", post_id)
        .replace("<!--content-->", content.post.content.as_str())
        .replace("<!--author-->", username.as_ref())
        .replace("<!--replies-->", replies_html.as_str());

    let login_regex = Regex::new(r"<!--startLogin-->(.|\n)*<!--endLogin-->").unwrap();
    let logout_regex = Regex::new(r"<!--startLogout-->(.|\n)*<!--endLogout-->").unwrap();
    let post_regex = Regex::new(r"<!--createPostUIStart-->(.|\n)*<!--createPostUIEnd-->").unwrap();
    response = match user {
        Some(user) => {
            response = response.replace("<!--username-->", user.account.username.as_str());
            login_regex.replace_all(&response, "").into_owned()
        }
        None => {
            response = response.replace("<!--username-->", "");
            response = logout_regex.replace_all(&response, "").into_owned();
            post_regex.replace_all(&response, "").into_owned()
        }
    };

    let html = match is_login_error {
        true => response.replace(
            "<!--loginError-->",
            include_str!("html/templates/login-error.html"),
        ),
        false => response.replace("<!--loginError-->", ""),
    };
    Response::from_html(html)
}
