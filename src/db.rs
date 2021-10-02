use super::crypto_helpers;
use super::user_obj;
use futures::future::join_all;
use serde_json;
use uuid::Uuid;
use worker::*;

pub async fn get_content(env: &Env, post_id: &str) -> Result<Option<String>> {
    let prefix = get_prefix(post_id, 0);

    // get data
    let data = env.kv("POSTS")?.get(prefix.as_str()).await?;

    // convert to string and return
    let content: Option<String>;
    if let Some(contents) = data {
        content = Some(contents.as_string());
    } else {
        content = None;
    }

    Ok(content)
}

pub async fn post_content(env: &Env, post_id: &str, contents: &str) -> Result<()> {
    let kv = env.kv("POSTS")?;
    let prefix = get_prefix(post_id, 0);
    kv.put(prefix.as_str(), contents)?.execute().await?;
    Ok(())
}

pub async fn get_replies(env: &Env, post_id: &str) -> Result<Vec<(String, String)>> {
    let limit = 50;
    let prefix = get_prefix(post_id, 1);

    // get list of keys with correct prefix
    let keys = env
        .kv("POSTS")?
        .list()
        .prefix(prefix)
        .limit(limit)
        .execute()
        .await?;
    let kv = env.kv("POSTS")?;

    // get content for each key
    let replies = keys
        .keys
        .iter()
        .filter(|key| {
            if let Some(_) = key.name.rfind(|c: char| !c.is_whitespace()) {
                true
            } else {
                false
            }
        })
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
    let test = join_all(replies).await;
    Ok(test)
}

/*
 *  add zeros to prefix to ensure it is in the correct format e.g. right-justified
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

    let sessions_kv = env.kv("SESSIONS")?;
    let user_data = get_user(env, username).await?;
    match user_data {
        None => {
            return Ok(None);
        }
        Some(user) => {
            if crypto_helpers::verify_password(password, &user.hash) {
                let session_id = Uuid::new_v4().to_simple().to_string();
                sessions_kv.put(&session_id, username)?.execute().await?;
                return Ok(Some(session_id));
            } else {
                return Ok(None);
            }
        }
    }
}

async fn get_user<S: AsRef<str>>(env: &Env, user_id: S) -> Result<Option<user_obj::UserAccount>> {
    let user_id = user_id.as_ref();
    let users_kv = env.kv("USERS")?;
    let user_data = users_kv.get(user_id).await?;
    match user_data {
        Some(data) => {
            let deserialised: user_obj::UserAccount =
                serde_json::from_str(data.as_string().as_str())?;
            Ok(Some(deserialised))
        }
        None => Ok(None),
    }
}

pub async fn get_session<S: AsRef<str>>(
    env: Env,
    session_id: S,
) -> Result<Option<user_obj::UserAccount>> {
    let session_id = session_id.as_ref();
    todo!()
}

pub async fn create_user<S: AsRef<str>>(
    env: Env,
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

    let users_kv = env.kv("USERS")?;
    users_kv.put(username, serialized)?.execute().await?;
    Ok(Some(username.to_string()))
}
