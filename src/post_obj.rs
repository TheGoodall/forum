use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Post {
    pub user: String,
    pub content: String,
}

pub struct PostTitle {
    pub title: String,
    pub post: Post,
}
