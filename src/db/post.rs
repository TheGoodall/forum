use crate::db::user;
use crate::post_obj;

use crate::user_obj;
use futures::stream::FuturesOrdered;
use futures::StreamExt;
use worker::*;

pub async fn get_content(env: &Env, post_id: &str) -> Result<Option<post_obj::PostTitle>> {
    let prefix = get_prefix(post_id, 0);

    // get data
    let data = env.kv("POSTS")?.get(prefix.as_str()).await?;
    match data {
        None => Ok(None),
        Some(content) => {
            let post: post_obj::Post = serde_json::from_str(content.as_string().as_str())?;
            let user: Option<user_obj::User> = user::get_user(env, &post.user).await?;
            Ok(Some(post_obj::PostTitle {
                title: post_id.to_string(),
                post,
                user,
            }))
        }
    }

    // convert to string and return
}

pub async fn post_content(
    env: &Env,
    post_id: &str,
    contents: &str,
    user: user_obj::User,
) -> Result<()> {
    let kv = env.kv("POSTS")?;
    let prefix = get_prefix(post_id, 0);

    let contents = html_escape::encode_text(contents);

    let post = post_obj::Post {
        user: user.user_id,
        content: contents.to_string(),
    };
    let post_string = serde_json::to_string(&post)?;
    kv.put(prefix.as_str(), post_string)?.execute().await?;
    Ok(())
}

pub async fn get_replies(env: &Env, post_id: &str) -> Result<Vec<post_obj::PostTitle>> {
    let prefix = get_prefix(post_id, 1);

    // get list of keys with correct prefix
    let keys = env.kv("POSTS")?.list().prefix(prefix).execute().await?;

    // get content for each key
    let values = keys
        .keys
        .iter()
        // Ignore the case when the entire key is whitespace e.g. root post (root post is never a
        // child of another post)
        .filter(|key| key.name.rfind(|c: char| !c.is_whitespace()).is_some())
        .map(|key| async move {
            let key_name = key.name.as_str().trim_start();
            let kv = env.kv("POSTS")?;
            if let Some(body) = kv.get(key.name.as_str()).await? {
                let body = body.as_string();
                let post: post_obj::Post = serde_json::from_str(body.as_str())?;
                let user = post.user.to_string();
                let post_title = post_obj::PostTitle {
                    title: key_name.to_string(),
                    post,
                    user: user::get_user(env, user).await?,
                };
                return worker::Result::Ok(post_title);
            }
            Err(worker::Error::RustError(String::from(
                "Key is apparenly None",
            )))
        })
        // Actually run the created futures and convert back to iterator
        .collect::<FuturesOrdered<_>>()
        .filter_map(|v| async { v.ok() })
        .collect::<Vec<_>>()
        .await;
    Ok(values)
}

/*
 *  add zeros to prefix to ensure post_id is in the correct format e.g. right-justified
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

pub async fn delete_post(env: &Env, post_id: &str) -> Result<()> {
    let kv = env.kv("POSTS")?;
    let post_id = get_prefix(post_id, 0);
    kv.delete(&post_id).await?;
    Ok(())
}
