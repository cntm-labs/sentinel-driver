fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok()
}

macro_rules! require_pg {
    () => {
        match database_url() {
            Some(url) => url,
            None => return,
        }
    };
}

#[tokio::test]
async fn test_connect() {
    let _url = require_pg!();
    // TODO: implement when Connection is wired to live PG
}
