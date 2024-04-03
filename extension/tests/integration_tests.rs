mod util;
use rand::Rng;
use util::common;

// Integration tests are ignored by default
#[ignore]
#[tokio::test]
async fn test_scheduled_job() {
    let conn = common::init_database().await;
    let mut rng = rand::thread_rng();
    let test_num = rng.gen_range(1..100000);
    let test_table_name = format!("products_test_{}", test_num);
    common::init_test_table(&test_table_name, &conn).await;
    let job_name = format!("job_{}", test_num);

    common::init_embedding_svc_url(&conn).await;

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

    // should be exactly 1 job in the queue
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
async fn test_scheduled_single_table() {
    let conn = common::init_database().await;
    let mut rng = rand::thread_rng();
    let test_num = rng.gen_range(1..100000);
    let test_table_name = format!("products_test_{}", test_num);
    common::init_test_table(&test_table_name, &conn).await;
    let job_name = format!("job_{}", test_num);

    common::init_embedding_svc_url(&conn).await;

    // initialize a job
    let _ = sqlx::query(&format!(
        "SELECT vectorize.table(
        job_name => '{job_name}',
        \"table\" => '{test_table_name}',
        primary_key => 'product_id',
        columns => ARRAY['product_name'],
        transformer => 'all-MiniLM-L12-v2',
        table_method => 'append',
        schedule => '* * * * *'
    );"
    ))
    .execute(&conn)
    .await
    .expect("failed to init job");

    // should be exactly 1 job in the queue
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
async fn test_realtime_append_fail() {
    let conn = common::init_database().await;
    let mut rng = rand::thread_rng();
    let test_num = rng.gen_range(1..100000);
    let test_table_name = format!("products_test_{}", test_num);
    common::init_test_table(&test_table_name, &conn).await;
    let job_name = format!("job_{}", test_num);

    common::init_embedding_svc_url(&conn).await;
    // initialize a job
    let result = sqlx::query(&format!(
        "SELECT vectorize.table(
        job_name => '{job_name}',
        \"table\" => '{test_table_name}',
        primary_key => 'product_id',
        columns => ARRAY['product_name'],
        transformer => 'all-MiniLM-L12-v2',
        table_method => 'append',
        schedule => 'realtime'
    );"
    ))
    .execute(&conn)
    .await;
    // realtime + append is not supported
    assert!(result.is_err());
}

#[ignore]
#[tokio::test]
async fn test_realtime_job() {
    let conn = common::init_database().await;
    common::init_embedding_svc_url(&conn).await;
    let mut rng = rand::thread_rng();
    let test_num = rng.gen_range(1..100000);
    let test_table_name = format!("products_test_{}", test_num);
    common::init_test_table(&test_table_name, &conn).await;
    let job_name = format!("job_{}", test_num);

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

    let search_results =
        common::search_with_retry(&conn, "mobile devices", &job_name, 10, 2, 3, None)
            .await
            .expect("failed to exec search");
    assert_eq!(search_results.len(), 3);

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

    // index will need to rebuild
    tokio::time::sleep(tokio::time::Duration::from_secs(5 as u64)).await;
    let search_results =
        common::search_with_retry(&conn, "car testing devices", &job_name, 10, 2, 3, None)
            .await
            .expect("failed to exec search");

    let mut found_it = false;
    for row in search_results {
        let row: common::SearchResult = serde_json::from_value(row.search_results).unwrap();
        if row.product_id == random_product_id {
            assert_eq!(row.product_name, "car tester");
            found_it = true;
        }
    }
    assert!(found_it);
}

#[ignore]
#[tokio::test]
async fn test_rag() {
    let conn = common::init_database().await;
    common::init_embedding_svc_url(&conn).await;
    let mut rng = rand::thread_rng();
    let test_num = rng.gen_range(1..100000);
    let test_table_name = format!("products_test_{}", test_num);
    common::init_test_table(&test_table_name, &conn).await;
    let agent_name = format!("agent_{}", test_num);

    // initialize
    let _ = sqlx::query(&format!(
        "SELECT vectorize.init_rag(
            agent_name => '{agent_name}',
            table_name => '{test_table_name}',
            unique_record_id => 'product_id',
            \"column\" => 'description',
            transformer => 'sentence-transformers/all-MiniLM-L12-v2'
    );"
    ))
    .execute(&conn)
    .await
    .expect("failed to init job");

    // must be able to conduct vector search on agent tables
    let search_results =
        common::search_with_retry(&conn, "mobile devices", &agent_name, 10, 2, 3, None)
            .await
            .expect("failed to exec search");
    assert_eq!(search_results.len(), 3);
}

#[ignore]
#[tokio::test]
async fn test_rag_alternate_schema() {
    let conn = common::init_database().await;
    common::init_embedding_svc_url(&conn).await;
    let mut rng = rand::thread_rng();
    let test_num = rng.gen_range(1..100000);
    let test_table_name = format!("products_test_{}", test_num);
    common::init_test_table(&test_table_name, &conn).await;
    let agent_name = format!("agent_{}", test_num);

    // initialize
    let _ = sqlx::query(&format!(
        "SELECT vectorize.init_rag(
            agent_name => '{agent_name}',
            table_name => '{test_table_name}',
            unique_record_id => 'product_id',
            \"column\" => 'description',
            transformer => 'sentence-transformers/all-MiniLM-L12-v2'
    );"
    ))
    .execute(&conn)
    .await
    .expect("failed to init job");

    // must be able to conduct vector search on agent tables
    let search_results =
        common::search_with_retry(&conn, "mobile devices", &agent_name, 10, 2, 3, None)
            .await
            .expect("failed to exec search");
    assert_eq!(search_results.len(), 3);
}

#[ignore]
#[tokio::test]
async fn test_static() {
    // a static test. intended for use across extension version updates
    // run this test on one branch, update the extension version, then run it again
    let conn = common::init_database().await;
    common::init_embedding_svc_url(&conn).await;
    let mut rng = rand::thread_rng();
    let test_table_name = "products_test_static";
    common::init_test_table(&test_table_name, &conn).await;
    let job_name = "static_test_job";

    // initialize a job
    let _ = sqlx::query(&format!(
        "SELECT vectorize.table(
        job_name => '{job_name}',
        \"table\" => '{test_table_name}',
        primary_key => 'product_id',
        columns => ARRAY['product_name', 'description'],
        transformer => 'all-MiniLM-L12-v2',
        schedule => 'realtime',
        table_method => 'join'
    );"
    ))
    .execute(&conn)
    .await
    .expect("failed to init job");

    // TEST BASIC SEARCH FUNCTIONALITY
    let search_results =
        common::search_with_retry(&conn, "mobile devices", &job_name, 10, 2, 3, None)
            .await
            .expect("failed to exec search");
    assert_eq!(search_results.len(), 3);

    // TEST INSERT TRIGGER
    let random_product_id = rng.gen_range(1..100000);

    let insert_query = format!(
        "INSERT INTO \"{test_table_name}\"(product_id, product_name, description)
        VALUES ({random_product_id}, 'car tester', 'a product for testing cars');"
    );

    // insert a new row
    let _result = sqlx::query(&insert_query)
        .execute(&conn)
        .await
        .expect("failed to insert into test_table");

    // index will need to rebuild
    tokio::time::sleep(tokio::time::Duration::from_secs(5 as u64)).await;
    let search_results =
        common::search_with_retry(&conn, "car testing products", &job_name, 10, 2, 3, None)
            .await
            .expect("failed to exec search");

    let mut found_it = false;
    for row in search_results.iter() {
        let row: common::SearchResult = serde_json::from_value(row.search_results.clone()).unwrap();
        if row.product_id == random_product_id {
            assert_eq!(row.product_name, "car tester");
            found_it = true;
        }
    }
    assert!(found_it, "resulting records: {:?}", search_results);

    // TEST UPDATE TRIGGER
    let update_query = format!(
        "UPDATE \"{test_table_name}\" SET
            product_name = 'cat food', description = 'a product for feeding cats'
        WHERE product_id = {random_product_id};"
    );
    let _result = sqlx::query(&update_query)
        .execute(&conn)
        .await
        .expect("failed to insert into test_table");
    // index will need to rebuild
    tokio::time::sleep(tokio::time::Duration::from_secs(5 as u64)).await;
    let search_results = common::search_with_retry(&conn, "cat food", &job_name, 10, 2, 20, None)
        .await
        .expect("failed to exec search");

    let mut found_it = false;
    for row in search_results.iter() {
        let row: common::SearchResult = serde_json::from_value(row.search_results.clone()).unwrap();
        if row.product_id == random_product_id {
            assert_eq!(row.product_name, "cat food");
            assert_eq!(row.description, "a product for feeding cats");
            found_it = true;
        }
    }
    assert!(found_it, "resulting records: {:?}", search_results);
}

#[ignore]
#[tokio::test]
async fn test_realtime_tabled() {
    let conn = common::init_database().await;
    common::init_embedding_svc_url(&conn).await;
    let mut rng = rand::thread_rng();
    let test_num = rng.gen_range(1..100000);
    let test_table_name = format!("products_test_{}", test_num);
    common::init_test_table(&test_table_name, &conn).await;
    let job_name = format!("job_{}", test_num);

    // initialize a job
    let _ = sqlx::query(&format!(
        "SELECT vectorize.table(
        job_name => '{job_name}',
        \"table\" => '{test_table_name}',
        primary_key => 'product_id',
        columns => ARRAY['product_name'],
        transformer => 'all-MiniLM-L12-v2',
        schedule => 'realtime',
        table_method => 'join'
    );"
    ))
    .execute(&conn)
    .await
    .expect("failed to init job");

    let search_results =
        common::search_with_retry(&conn, "mobile devices", &job_name, 10, 2, 3, None)
            .await
            .expect("failed to exec search");
    assert_eq!(search_results.len(), 3);

    let random_product_id = rng.gen_range(0..100000);

    // insert a new row
    let insert_query = format!(
        "INSERT INTO \"{test_table_name}\"(product_id, product_name, description)
        VALUES ({random_product_id}, 'car tester', 'a product for testing cars');"
    );
    let _result = sqlx::query(&insert_query)
        .execute(&conn)
        .await
        .expect("failed to insert into test_table");

    // index will need to rebuild
    tokio::time::sleep(tokio::time::Duration::from_secs(5 as u64)).await;
    let search_results =
        common::search_with_retry(&conn, "car testing devices", &job_name, 10, 2, 3, None)
            .await
            .expect("failed to exec search");

    let mut found_it = false;
    for row in search_results {
        let row: common::SearchResult = serde_json::from_value(row.search_results).unwrap();
        if row.product_id == random_product_id {
            assert_eq!(row.product_name, "car tester");
            found_it = true;
        }
    }
    assert!(found_it);

    // `join` method must have a view created
    let select = format!(
        "SELECT product_id, product_name, description, embeddings, embeddings_updated_at FROM vectorize.{job_name}"
    );
    let result = sqlx::query(&select)
        .fetch_all(&conn)
        .await
        .expect("failed to query project view");

    // 41 rows should be returned
    assert!(result.len() == 41);
}

#[ignore]
#[tokio::test]
async fn test_filter_join() {
    let conn = common::init_database().await;
    common::init_embedding_svc_url(&conn).await;
    let mut rng = rand::thread_rng();
    let test_num = rng.gen_range(1..100000);
    let test_table_name = format!("products_test_{}", test_num);
    common::init_test_table(&test_table_name, &conn).await;
    let job_name = format!("job_{}", test_num);

    // initialize a job using `join` method
    let _ = sqlx::query(&format!(
        "SELECT vectorize.table(
        job_name => '{job_name}',
        \"table\" => '{test_table_name}',
        primary_key => 'product_id',
        columns => ARRAY['product_name'],
        transformer => 'all-MiniLM-L12-v2',
        schedule => 'realtime',
        table_method => 'join'
    );"
    ))
    .execute(&conn)
    .await
    .expect("failed to init job");

    let filter = "product_id = 1".to_string();
    let search_results =
        common::search_with_retry(&conn, "mobile devices", &job_name, 10, 2, 1, Some(filter))
            .await
            .expect("failed to exec search");
    assert_eq!(search_results.len(), 1);
    let result_val = search_results[0].search_results.clone();
    let product_id_val = result_val["product_id"]
        .as_i64()
        .expect("failed parsing product id");
    assert_eq!(product_id_val, 1);

    let filter = "product_id = 1 and product_name ilike 'pencil'".to_string();
    let search_results = common::search_with_retry(
        &conn,
        "some random query",
        &job_name,
        10,
        2,
        1,
        Some(filter),
    )
    .await
    .expect("failed to exec search");
    assert_eq!(search_results.len(), 1);
    let result_val = search_results[0].search_results.clone();
    let product_id_val = result_val["product_id"]
        .as_i64()
        .expect("failed parsing product id");
    assert_eq!(product_id_val, 1);
}

#[ignore]
#[tokio::test]
async fn test_filter_append() {
    let conn = common::init_database().await;
    common::init_embedding_svc_url(&conn).await;
    let mut rng = rand::thread_rng();
    let test_num = rng.gen_range(1..100000);
    let test_table_name = format!("products_test_{}", test_num);
    common::init_test_table(&test_table_name, &conn).await;
    let job_name = format!("job_{}", test_num);

    // initialize a job using `join` method
    let _ = sqlx::query(&format!(
        "SELECT vectorize.table(
        job_name => '{job_name}',
        \"table\" => '{test_table_name}',
        primary_key => 'product_id',
        columns => ARRAY['product_name'],
        transformer => 'all-MiniLM-L12-v2',
        table_method => 'append'
    );"
    ))
    .execute(&conn)
    .await
    .expect("failed to init job");

    let filter = "product_id = 2".to_string();
    let search_results =
        common::search_with_retry(&conn, "mobile devices", &job_name, 10, 2, 1, Some(filter))
            .await
            .expect("failed to exec search");
    assert_eq!(search_results.len(), 1);
    let result_val = search_results[0].search_results.clone();
    let product_id_val = result_val["product_id"]
        .as_i64()
        .expect("failed parsing product id");
    assert_eq!(product_id_val, 2);
}

#[tokio::test]
async fn test_index_dist_type() {
    let conn = common::init_database().await;
    common::init_embedding_svc_url(&conn).await;

    let dist_types = vec![
        ("pgv_hnsw_cosine", true), // Expected to pass
        ("pgv_hnsw_l2", false),    // Expected to fail
        ("pgv_hnsw_ip", false),    // Expected to fail
    ];

    for (dist_type, should_pass) in dist_types {
        let mut rng = rand::thread_rng();
        let test_num = rng.gen_range(1..100000);
        let test_table_name = format!("products_test_{}", test_num);
        let job_name = format!("job_{}", test_num);

        common::init_test_table(&test_table_name, &conn).await;

        let query = format!(
            "SELECT vectorize.table(
            job_name => '{job_name}',
            \"table\" => '{test_table_name}',
            primary_key => 'product_id',
            columns => ARRAY['product_name'],
            index_dist_type => '{dist_type}',
            transformer => 'all-MiniLM-L12-v2',
            schedule => 'realtime'
        );",
            job_name = job_name,
            test_table_name = test_table_name,
            dist_type = dist_type,
        );

        let result = sqlx::query(&query).execute(&conn).await;

        // Assert based on whether this index_dist_type is expected to pass or fail
        match (should_pass, result) {
            (true, Ok(_)) => (),   // Pass: Expected to succeed, and it did.
            (false, Err(_)) => (), // Pass: Expected to fail, and it did.
            (true, Err(e)) => panic!("Expected to pass, but failed: {:?}", e),
            (false, Ok(_)) => panic!("Expected to fail, but passed."),
        }

        if should_pass {
            let filter = "product_id = 2".to_string();
            let search_results = common::search_with_retry(
                &conn,
                "mobile devices",
                &job_name,
                10,
                2,
                1,
                Some(filter),
            )
            .await
            .expect("failed to exec search");
            assert_eq!(search_results.len(), 1);

            let result_val = search_results[0].search_results.clone();
            let product_id_val = result_val["product_id"]
                .as_i64()
                .expect("failed parsing product id");
            assert_eq!(product_id_val, 2);
        }
    }
}
