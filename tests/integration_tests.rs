use rand::Rng;
mod util;
use sqlx::FromRow;

use util::common;

// Integration tests are ignored by default
#[ignore]
#[tokio::test]
async fn test_scheduled_job() {
    let conn = common::init_database().await;
    let mut rng = rand::thread_rng();
    let test_num = rng.gen_range(0..100000);
    let test_table_name = common::init_test_table(test_num, &conn).await;
    let job_name = format!("job_{}", test_num);

    common::init_embedding_svc_url(&conn).await;
    println!("test_table_name: {}", test_table_name);
    println!("job_name: {}", job_name);
    // initialize a job
    let _ = sqlx::query(&format!(
        "SELECT vectorize.table(
        job_name => '{job_name}',
        \"table\" => '{test_table_name}',
        primary_key => 'product_id',
        columns => ARRAY['product_name'],
        transformer => 'all-MiniLM-L12-v2',
        schedule => '* * * * *'
    );"
    ))
    .execute(&conn)
    .await
    .expect("failed to init job");

    // manually trigger a job
    let _ = sqlx::query(&format!("SELECT vectorize.job_execute('{job_name}');"))
        .execute(&conn)
        .await
        .expect("failed to select from test_table");

    // should 1 job in the queue
    let rowcount = common::row_count(&format!("pgmq.q_vectorize_jobs"), &conn).await;
    assert!(rowcount >= 1);

    // embedding should be updated after few seconds
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    let result = sqlx::query(&format!(
        "SELECT vectorize.search(
        job_name => '{job_name}',
        query => 'mobile devices',
        return_columns => ARRAY['product_name'],
        num_results => 3
    );"
    ))
    .execute(&conn)
    .await
    .expect("failed to select from test_table");
    // 3 rows returned
    assert_eq!(result.rows_affected(), 3);
}

#[ignore]
#[tokio::test]
async fn test_realtime_job() {
    let conn = common::init_database().await;
    common::init_embedding_svc_url(&conn).await;
    let mut rng = rand::thread_rng();
    let test_num = rng.gen_range(0..100000);
    let test_table_name = common::init_test_table(test_num, &conn).await;
    let job_name = format!("job_{}", test_num);

    println!("test_table_name: {}", test_table_name);
    println!("job_name: {}", job_name);
    // initialize a job
    let _ = sqlx::query(&format!(
        "SELECT vectorize.table(
        job_name => '{job_name}',
        \"table\" => '{test_table_name}',
        primary_key => 'product_id',
        columns => ARRAY['product_name'],
        transformer => 'all-MiniLM-L12-v2',
        schedule => 'realtime'
    );"
    ))
    .execute(&conn)
    .await
    .expect("failed to init job");

    // embedding should be updated after few seconds
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    let result = sqlx::query(&format!(
        "SELECT vectorize.search(
        job_name => '{job_name}',
        query => 'mobile devices',
        return_columns => ARRAY['product_name', 'product_id'],
        num_results => 3
    );"
    ))
    .execute(&conn)
    .await
    .expect("failed to select from test_table");
    // 3 rows returned
    assert_eq!(result.rows_affected(), 3);

    let random_product_id = rng.gen_range(0..100000);

    let insert_query = format!(
        "INSERT INTO \"{test_table_name}\"(product_id, product_name, description)
        VALUES ({random_product_id}, 'car tester', 'a product for testing cars');"
    );

    // insert a new row
    let _result = sqlx::query(&insert_query)
        .execute(&conn)
        .await
        .expect("failed to insert into test_table");

    // embedding should be updated after few seconds
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    #[allow(dead_code)]
    #[derive(FromRow, Debug, serde::Deserialize)]
    struct SearchResult {
        product_id: i32,
        product_name: String,
        similarity_score: f64,
    }

    #[allow(dead_code)]
    #[derive(FromRow, Debug)]
    struct SearchJSON {
        search_results: serde_json::Value,
    }

    let result = sqlx::query_as::<_, SearchJSON>(&format!(
        "SELECT search_results from vectorize.search(
        job_name => '{job_name}',
        query => 'car testing devices',
        return_columns => ARRAY['product_id','product_name'],
        num_results => 3
    ) as search_results;"
    ))
    .fetch_all(&conn)
    .await
    .expect("failed to execute search");

    let mut found_it = false;
    for row in result {
        println!("row: {:?}", row);
        let row: SearchResult = serde_json::from_value(row.search_results).unwrap();
        if row.product_id == random_product_id {
            assert_eq!(row.product_name, "car tester");
            found_it = true;
        }
    }
    assert!(found_it);
}
