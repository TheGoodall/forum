use crate::db::post::*;
use crate::user_obj;
use regex::Regex;
use worker::*;

pub async fn render_page(
    path: &str,
    env: &Env,
    is_login_error: bool,
    user: Option<user_obj::User>,
) -> Result<Response> {

    let styles = [
        include_str!("html/style/login.css"),
        include_str!("html/style/layout.css"),
        include_str!("html/style/index.css")
    ];

    let style = styles.join("\n");

    // Get post id from path
    let post_id = path
        .strip_prefix('/')
        .expect("Expected path to begin with /");

    let prev_post_id = &post_id[..if post_id.chars().count() > 0 {
        post_id.chars().count() - 1
    } else {
        0
    }];

    // get content, return error if page doesn't exists
    let content = match get_content(env, post_id).await? {
        None => {
            return Response::error("Page Not Found", 404);
        }
        Some(content) => content,
    };

    // get all replies to post
    let replies = get_replies(env, post_id).await?;

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

    let author_username = match &content.user {
        Some(user) => &user.account.username,
        None => "[Deleted]",
    };
    let author_userid = match &content.user {
        Some(user) => &user.user_id,
        None => "[Deleted]",
    };

    let mut response = include_str!("html/index.html")
        .replace("/*style*/", style.as_str())
        .replace("<!--title-->", post_id)
        .replace("<!--content-->", content.post.content.as_str())
        .replace("<!--author-->", author_username.as_ref())
        .replace("<!--replies-->", replies_html.as_str())
        .replace("<!--backPath-->", prev_post_id);

    let login_regex = Regex::new(r"<!--startLogin-->(.|\n)*<!--endLogin-->").unwrap();
    let logout_regex = Regex::new(r"<!--startLogout-->(.|\n)*<!--endLogout-->").unwrap();
    let post_regex = Regex::new(r"<!--createPostUIStart-->(.|\n)*<!--createPostUIEnd-->").unwrap();
    let edit_regex = Regex::new(r"<!--editPostUIStart-->(.|\n)*<!--editPostUIEnd-->").unwrap();
    response = match user {
        Some(user) => {
            response = response.replace("<!--username-->", user.account.username.as_str());
            response = login_regex.replace_all(&response, "").into_owned();
            if user.user_id != author_userid {
                response = edit_regex.replace_all(&response, "").into_owned();
            }
            response
        }
        None => {
            response = response.replace("<!--username-->", "");
            response = logout_regex.replace_all(&response, "").into_owned();
            response = edit_regex.replace_all(&response, "").into_owned();
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
