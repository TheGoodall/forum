use crate::user_obj;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Post {
    pub user: String,
    pub content: String,
}

pub struct PostTitle {
    pub title: String,
    pub user: Option<user_obj::User>,
    pub post: Post,
}
