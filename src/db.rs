use futures::future::join_all;
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
    env: Env,
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
        Some(user) => {}
    }

    let session_id = Uuid::new_v4().to_simple().to_string();
    todo!()
}

pub struct User {
    email: String,
}

pub struct UserData {
    password: String,
}

async fn get_user<S: AsRef<str>>(env: Env, user_id: S) -> Result<Option<User>> {
    let user_id = user_id.as_ref();
    let users_kv = env.kv("USERS")?;
    let user_data = users_kv.get(user_id).await?;
    let user_object = match user_data {
        Some(data) => {
            let deserialised = deserialise_user_data(data.as_string());
            Ok(Some(User {
                email: user_id.to_string(),
            }))
        }
        None => return Ok(None),
    };
    user_object
}

fn deserialise_user_data<S: AsRef<str>>(user_data: S) -> UserData {
    todo!()
}

pub async fn get_session<S: AsRef<str>>(env: Env, session_id: S) -> Result<Option<User>> {
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
    todo!()
}
