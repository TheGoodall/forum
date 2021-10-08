use crate::crypto_helpers;
use crate::user_obj;
use uuid::Uuid;
use worker::*;

pub async fn create_session<S: AsRef<str>>(
    env: &Env,
    user_id: S,
    password: S,
) -> Result<Option<String>> {
    let user_id = user_id.as_ref();
    let password = password.as_ref();

    match get_user(env, user_id).await? {
        None => Ok(None),
        Some(user) => {
            if crypto_helpers::verify_password(password, &user.account.hash) {
                let session_id = Uuid::new_v4().to_simple().to_string();
                update_session(env, user_id, &session_id).await?;

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
    user_id: S,
    session_id: S2,
) -> Result<()> {
    let sessions_kv = env.kv("SESSIONS")?;

    let expiry: u64 = env
        .var("SESSION_EXPIRY")?
        .to_string()
        .parse::<u64>()
        .expect("Error: Could not parse expiry environment variable");

    sessions_kv
        .put(session_id.as_ref(), user_id.as_ref())?
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

pub async fn get_user<S: AsRef<str>>(env: &Env, user_id: S) -> Result<Option<user_obj::User>> {
    let user_id = user_id.as_ref();
    let users_kv = env.kv("USERS")?;
    let user_data = users_kv.get(user_id).await?;
    Ok(match user_data {
        Some(data) => {
            let deserialised: user_obj::UserAccount =
                serde_json::from_str(data.as_string().as_str())?;
            Some(user_obj::User {
                account: deserialised,
                user_id: user_id.to_string(),
            })
        }
        None => None,
    })
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
    user_id: S,
    username: S,
    password: S,
) -> Result<Option<String>> {
    let username = username.as_ref();
    let password = password.as_ref();
    let user_id = user_id.as_ref();

    let hash = crypto_helpers::hash_password(password);
    let acc = user_obj::UserAccount {
        hash: hash.to_string(),
        username: username.to_string(),
    };
    let serialized = serde_json::to_string(&acc).unwrap();

    if get_user(env, user_id).await?.is_some() {
        return Ok(None);
    }

    let users_kv = env.kv("USERS")?;
    users_kv.put(user_id, serialized)?.execute().await?;

    let session_id = create_session(env, user_id, password)
        .await?
        .expect("Create session failed when it shouldn't have");
    Ok(Some(session_id))
}
