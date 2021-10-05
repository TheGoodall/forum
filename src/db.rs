use crate::post_obj;

use super::crypto_helpers;
use super::user_obj;
use futures::future::join_all;
use uuid::Uuid;
use worker::*;

pub async fn get_content(env: &Env, post_id: &str) -> Result<Option<post_obj::PostTitle>> {
    let prefix = get_prefix(post_id, 0);

    // get data
    let data = env.kv("POSTS")?.get(prefix.as_str()).await?;
    match data {
        None => Ok(None),
        Some(content) => {
            let post = serde_json::from_str(content.as_string().as_str())?;
            Ok(Some(post_obj::PostTitle {
                title: post_id.to_string(),
                post,
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
    let kv = env.kv("POSTS")?;

    // get content for each key
    let replies = keys
        .keys
        .iter()
        // Ignore the case when the entire key is whitespace e.g. root post (root post is never a
        // child of another post)
        .filter(|key| key.name.rfind(|c: char| !c.is_whitespace()).is_some())
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
    // Perform all IO async
    let reply_pairs = join_all(replies).await;
    Ok(reply_pairs
        .iter()
        .map(|(title, body)| -> Result<post_obj::PostTitle> {
            Ok(post_obj::PostTitle {
                title: title.to_string(),
                post: serde_json::from_str(body)?,
            })
        })
        .filter_map(|value| value.ok())
        .collect())
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

pub async fn create_session<S: AsRef<str>>(
    env: &Env,
    username: S,
    password: S,
) -> Result<Option<String>> {
    let username = username.as_ref();
    let password = password.as_ref();

    match get_user(env, username).await? {
        None => Ok(None),
        Some(user) => {
            if crypto_helpers::verify_password(password, &user.account.hash) {
                let session_id = Uuid::new_v4().to_simple().to_string();
                update_session(env, username, &session_id).await?;

                Ok(Some(session_id))
            } else {
                Ok(None)
            }
        }
    }
}
/*
 * Write the session to the kv store with the correct expiry time
 * */
async fn update_session<S: AsRef<str>, S2: AsRef<str>>(
    env: &Env,
    username: S,
    session_id: S2,
) -> Result<()> {
    let sessions_kv = env.kv("SESSIONS")?;

    let expiry: u64 = env
        .var("SESSION_EXPIRY")?
        .to_string()
        .parse::<u64>()
        .expect("Error: Could not parse expiry environment variable");

    sessions_kv
        .put(session_id.as_ref(), username.as_ref())?
        .expiration_ttl(expiry)
        .execute()
        .await?;

    Ok(())
}

pub async fn delete_session<S: AsRef<str>>(env: &Env, session_id: S) -> Result<()> {
    let session_id = session_id.as_ref();
    let sessions_kv = env.kv("SESSIONS")?;
    sessions_kv.delete(session_id).await?;
    Ok(())
}

async fn get_user<S: AsRef<str>>(env: &Env, user_id: S) -> Result<Option<user_obj::User>> {
    let user_id = user_id.as_ref();
    let users_kv = env.kv("USERS")?;
    let user_data = users_kv.get(user_id).await?;
    match user_data {
        Some(data) => {
            let deserialised: user_obj::UserAccount =
                serde_json::from_str(data.as_string().as_str())?;
            Ok(Some(user_obj::User {
                account: deserialised,
                user_id: user_id.to_string(),
            }))
        }
        None => Ok(None),
    }
}

pub async fn get_session<S: AsRef<str>>(
    env: &Env,
    session_id: S,
) -> Result<Option<user_obj::User>> {
    let session_id = session_id.as_ref();

    let sessions_kv = env.kv("SESSIONS")?;

    match sessions_kv.get(session_id).await? {
        None => Ok(None),
        Some(user_id) => {
            let user_id = user_id.as_string();
            update_session(env, &user_id, session_id).await?;
            get_user(env, &user_id).await
        }
    }
}

pub async fn create_user<S: AsRef<str>>(
    env: &Env,
    username: S,
    password: S,
) -> Result<Option<String>> {
    let username = username.as_ref();
    let password = password.as_ref();

    let hash = crypto_helpers::hash_password(password);
    let acc = user_obj::UserAccount {
        hash: hash.to_string(),
    };
    let serialized = serde_json::to_string(&acc).unwrap();

    if get_user(env, username).await?.is_some() {
        return Ok(None);
    }

    let users_kv = env.kv("USERS")?;
    users_kv.put(username, serialized)?.execute().await?;

    let session_id = create_session(env, username, password)
        .await?
        .expect("Create session failed when it shouldn't have");
    Ok(Some(session_id))
}
