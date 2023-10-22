// use vectorize::util::{get_pg_options};

// use regex::Regex;
// use std::str::FromStr;

// #[derive(Debug, Default)]
// struct PostgresSocketConnection {
//     user: Option<String>,
//     dbname: Option<String>,
//     host: Option<String>,
//     password: Option<String>,
//     // Add other potential query parameters as needed
// }

// impl PostgresSocketConnection {
//     fn from_unix_socket_string(s: &str) -> Option<Self> {
//         let parsed_url = url::Url::parse(s).ok()?;
//         let mut connection = PostgresSocketConnection::default();

//         for (key, value) in parsed_url.query_pairs() {
//             match key.as_ref() {
//                 "user" => connection.user = Some(value.into_owned()),
//                 "dbname" => connection.dbname = Some(value.into_owned()),
//                 "host" => connection.host = Some(value.into_owned()),
//                 "password" => connection.password = Some(value.into_owned()),
//                 // Add other potential query parameters as needed
//                 _ => {} // Ignoring unknown parameters
//             }
//         }

//         Some(connection)
//     }
// }

#[tokio::main]
async fn main() {
    // let s = "postgresql:///postgres?host=/Users/adamhendel/.pgrx&user=postgress&dbname=postgres,password=pw".to_owned();
    // let connection = PostgresSocketConnection::from_unix_socket_string(&s);
    // println!("{:?}", connection);
    // let c = PostgresConnection::from_unix_socket_string(&s);
    // println!("{:?}", c);
    // let opts = get_pg_options().unwrap();
    // println!("{:?}", opts);
    // let _conn = get_pg_conn().await.unwrap();
    println!("Hello, world!");
}
