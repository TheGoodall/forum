use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct UserAccount {
    pub hash: String,
}

pub struct User {
    pub account: UserAccount,
    pub user_id: String,
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json;

    #[test]
    fn serialize_obj() {
        let hash = "1";
        let acc = UserAccount {
            hash: hash.to_string(),
        };
        let serialized = serde_json::to_string(&acc).unwrap();
        println!("{}", serialized);
        assert_ne!(serialized, "");
    }
}
