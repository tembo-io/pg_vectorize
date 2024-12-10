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
        transformer => 'sentence-transformers/all-MiniLM-L6-v2',
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
async fn test_drop_table_triggers_job_deletion() {
    let conn = common::init_database().await;
    let mut rng = rand::thread_rng();
    let test_num = rng.gen_range(1..100000);
    let test_table_name = format!("test_table_{}", test_num);
    let job_name = format!("job_{}", test_num);

    common::init_test_table(&test_table_name, &conn).await;
    common::init_embedding_svc_url(&conn).await;

    let create_job_query = format!(
        "SELECT vectorize.table(
            job_name => '{job_name}',
            \"table\" => '{test_table_name}',
            primary_key => 'product_id',
            columns => ARRAY['product_name'],
            transformer => 'sentence-transformers/all-MiniLM-L6-v2'
        );"
    );
    sqlx::query(&create_job_query)
        .bind(&job_name)
        .bind(&test_table_name)
        .execute(&conn)
        .await
        .expect("failed to create job");
    
    // Verify the job exists before dropping the table
    let job_count_before = common::row_count("vectorize.job", &conn).await;
    assert_eq!(
        job_count_before, 1,
        "Expected 1 job in vectorize.job, found {}",
        job_count_before
    );

    // Drop the test table with CASCADE to remove dependencies
    let drop_query = format!("DROP TABLE public.{test_table_name} CASCADE;");
    sqlx::query(&drop_query)
        .execute(&conn)
        .await
        .expect("Failed to drop test table");

    // Check row count in vectorize.job after dropping the table
    let job_count_after = common::row_count("vectorize.job", &conn).await;
    assert_eq!(
        job_count_after, 0,
        "Job was not deleted after dropping table"
    );

    // Verify the specific job no longer exists in vectorize.job
    let job_exists: bool = sqlx::query_scalar(&format!(
        "SELECT EXISTS (SELECT 1 FROM vectorize.job WHERE name = $1);"
    ))
    .bind(&job_name)
    .fetch_one(&conn)
    .await
    .expect("failed to check job existence");

    assert!(
        !job_exists,
        "Job associated with table `{}` was not deleted",
        test_table_name
    );
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
        transformer => 'sentence-transformers/all-MiniLM-L6-v2',
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
        transformer => 'sentence-transformers/all-MiniLM-L6-v2',
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
        columns => ARRAY['product_name', 'description'],
        transformer => 'sentence-transformers/all-MiniLM-L6-v2',
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
        "INSERT INTO \"{test_table_name}\"(product_id, product_name, description, product_category, price)
        VALUES ({random_product_id}, 'car tester', $$a product for testing car's components$$, 'electronics', 10.99);"
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
            assert_eq!(row.product_id, random_product_id);
            found_it = true;
        }
    }
    assert!(found_it);

    // test with some double dollar quote string data
    let random_product_id = rng.gen_range(0..100000);

    let insert_query = format!(
        "INSERT INTO \"{test_table_name}\"(product_id, product_name, description, product_category, price)
        VALUES ({random_product_id}, 'messy-product', $DELIM$the $$quick brown fox jump's over the lazy dog$DELIM$, 'product', 10.99);"
    );

    // insert a new row
    let _result = sqlx::query(&insert_query)
        .execute(&conn)
        .await
        .expect("failed to insert into test_table");

    // index will need to rebuild
    tokio::time::sleep(tokio::time::Duration::from_secs(5 as u64)).await;
    let search_results =
        common::search_with_retry(&conn, "messy-product", &job_name, 10, 2, 3, None)
            .await
            .expect("failed to exec search");

    let mut found_it = false;
    for row in search_results {
        let row: common::SearchResult = serde_json::from_value(row.search_results).unwrap();
        if row.product_id == random_product_id {
            assert_eq!(row.product_name, "messy-product");
            assert_eq!(row.product_id, random_product_id);
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
            transformer => 'sentence-transformers/all-MiniLM-L6-v2'
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
            transformer => 'sentence-transformers/all-MiniLM-L6-v2'
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
        transformer => 'sentence-transformers/all-MiniLM-L6-v2',
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
        "INSERT INTO \"{test_table_name}\"(product_id, product_name, description, product_category, price)
        VALUES ({random_product_id}, 'car tester', 'a product for testing cars', 'electronics', 10.99);"
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
        transformer => 'sentence-transformers/all-MiniLM-L6-v2',
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
        "INSERT INTO \"{test_table_name}\"(product_id, product_name, description, product_category, price)
        VALUES ({random_product_id}, 'car tester', 'a product for testing cars', 'electronics', 10.99);"
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
        "SELECT product_id, product_name, description, embeddings, embeddings_updated_at FROM vectorize.{job_name}_view"
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
        transformer => 'sentence-transformers/all-MiniLM-L6-v2',
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
        transformer => 'sentence-transformers/all-MiniLM-L6-v2',
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

#[ignore]
#[tokio::test]
async fn test_index_dist_type_hnsw_cosine() {
    let conn = common::init_database().await;
    common::init_embedding_svc_url(&conn).await;

    let dist_type = "pgv_hnsw_cosine";
    let test_num: u32 = rand::thread_rng().gen_range(1..100000);
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
        transformer => 'sentence-transformers/all-MiniLM-L6-v2',
        schedule => 'realtime'
    );",
        job_name = job_name,
        test_table_name = test_table_name,
        dist_type = dist_type,
    );

    let result = sqlx::query(&query).execute(&conn).await;

    assert!(
        result.is_ok(),
        "pgv_hnsw_cosine index_dist_type should pass but failed with: {:?}",
        result.err()
    );

    // Now perform a search operation to ensure it passes
    let search_query = format!(
        "SELECT * FROM vectorize.search(
            job_name => '{job_name}',
            query => 'search query',
            return_columns => ARRAY['product_name'],
            num_results => 10
        );",
        job_name = job_name
    );

    let search_result = sqlx::query(&search_query).fetch_all(&conn).await;

    // Assert that the search operation is successful
    assert!(
        search_result.is_ok(),
        "Search operation failed with pgv_hnsw_cosine distribution type: {:?}",
        search_result.err()
    );
}

#[tokio::test]
#[ignore]
async fn test_index_dist_type_hnsw_l2() {
    let conn = common::init_database().await;
    common::init_embedding_svc_url(&conn).await;

    let dist_type = "pgv_hnsw_l2";
    let test_num: u32 = rand::thread_rng().gen_range(1..100000);
    let test_table_name = format!("products_test_{}", test_num);
    let job_name = format!("job_{}", test_num);

    // Initialize the test table and job
    common::init_test_table(&test_table_name, &conn).await;
    let _ = sqlx::query(&format!(
        "SELECT vectorize.table(
            job_name => '{job_name}',
            \"table\" => '{test_table_name}',
            primary_key => 'product_id',
            columns => ARRAY['product_name'],
            index_dist_type => '{dist_type}',
            transformer => 'sentence-transformers/all-MiniLM-L6-v2',
            schedule => 'realtime'
        );",
        job_name = job_name,
        test_table_name = test_table_name,
        dist_type = dist_type,
    ))
    .execute(&conn)
    .await
    .expect("failed to initialize job");

    // Directly call vectorize.search to perform a search operation
    let search_query = format!(
        "SELECT * FROM vectorize.search(
            job_name => '{job_name}',
            query => 'search query',
            return_columns => ARRAY['product_name'],
            num_results => 10
        );",
        job_name = job_name
    );

    let search_result = sqlx::query(&search_query).fetch_all(&conn).await;

    // Assert that the search operation fails as expected with pgv_hnsw_l2
    assert!(
        search_result.is_err(),
        "Expected search with pgv_hnsw_l2 to fail, but it succeeded."
    );
}

#[tokio::test]
#[ignore]
async fn test_index_dist_type_hnsw_ip() {
    let conn = common::init_database().await;
    common::init_embedding_svc_url(&conn).await;

    let dist_type = "pgv_hnsw_ip";
    let test_num: u32 = rand::thread_rng().gen_range(1..100000);
    let test_table_name = format!("products_test_{}", test_num);
    let job_name = format!("job_{}", test_num);

    // Initialize the test table and job
    common::init_test_table(&test_table_name, &conn).await;
    let _ = sqlx::query(&format!(
        "SELECT vectorize.table(
            job_name => '{job_name}',
            \"table\" => '{test_table_name}',
            primary_key => 'product_id',
            columns => ARRAY['product_name'],
            index_dist_type => '{dist_type}',
            transformer => 'sentence-transformers/all-MiniLM-L6-v2',
            schedule => 'realtime'
        );",
        job_name = job_name,
        test_table_name = test_table_name,
        dist_type = dist_type,
    ))
    .execute(&conn)
    .await
    .expect("failed to initialize job");

    // Directly call vectorize.search to perform a search operation
    let search_query = format!(
        "SELECT * FROM vectorize.search(
            job_name => '{job_name}',
            query => 'search query',
            return_columns => ARRAY['product_name'],
            num_results => 10
        );",
        job_name = job_name
    );

    let search_result = sqlx::query(&search_query).fetch_all(&conn).await;

    // Assert that the search operation fails as expected with pgv_hnsw_l2
    assert!(
        search_result.is_err(),
        "Expected search with pgv_hnsw_ip to fail, but it succeeded."
    );
}

#[ignore]
#[tokio::test]
async fn test_private_hf_model() {
    let conn = common::init_database().await;
    let mut rng = rand::thread_rng();
    let test_num = rng.gen_range(1..100000);
    let test_table_name = format!("products_test_{}", test_num);
    common::init_test_table(&test_table_name, &conn).await;
    let job_name = format!("job_{}", test_num);

    common::init_embedding_svc_url(&conn).await;

    let hf_api_key = std::env::var("HF_API_KEY").expect("HF_API_KEY must be set");

    let mut tx = conn.begin().await.unwrap();

    sqlx::query(&format!(
        "set vectorize.embedding_service_api_key to '{hf_api_key}'"
    ))
    .execute(&mut *tx)
    .await
    .unwrap();

    // initialize a job
    let created = sqlx::query(&format!(
        "SELECT vectorize.table(
        job_name => '{job_name}',
        \"table\" => '{test_table_name}',
        primary_key => 'product_id',
        columns => ARRAY['product_name'],
        transformer => 'chuckhend/private-model',
        schedule => 'realtime'
    );"
    ))
    .execute(&mut *tx)
    .await;

    tx.commit().await.unwrap();

    assert!(created.is_ok(), "Failed with error: {:?}", created);

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
async fn test_diskann_cosine() {
    let conn = common::init_database().await;
    let mut rng = rand::thread_rng();
    let test_num = rng.gen_range(1..100000);
    let test_table_name = format!("products_test_{}", test_num);
    common::init_test_table(&test_table_name, &conn).await;
    let job_name = format!("job_diskann_{}", test_num);

    common::init_embedding_svc_url(&conn).await;
    // initialize a job
    let result = sqlx::query(&format!(
        "SELECT vectorize.table(
        job_name => '{job_name}',
        \"table\" => '{test_table_name}',
        primary_key => 'product_id',
        columns => ARRAY['product_name'],
        transformer => 'sentence-transformers/all-MiniLM-L6-v2',
        index_dist_type => 'vsc_diskann_cosine',
        schedule => 'realtime'
    );"
    ))
    .execute(&conn)
    .await;
    assert!(result.is_ok());

    let search_results: Vec<common::SearchJSON> =
        match util::common::search_with_retry(&conn, "mobile devices", &job_name, 10, 2, 3, None)
            .await
        {
            Ok(results) => results,
            Err(e) => {
                eprintln!("Error: {:?}", e);
                panic!("failed to exec search on diskann");
            }
        };
    assert_eq!(search_results.len(), 3);
}

#[ignore]
#[tokio::test]
async fn test_cohere() {
    let conn = common::init_database().await;
    let mut rng = rand::thread_rng();
    let test_num = rng.gen_range(1..100000);
    let test_table_name = format!("products_test_{}", test_num);
    common::init_test_table(&test_table_name, &conn).await;
    let job_name = format!("cohere_{}", test_num);

    let hf_api_key = std::env::var("CO_API_KEY").expect("CO_API_KEY must be set");

    let mut tx = conn.begin().await.unwrap();

    sqlx::query(&format!("set vectorize.cohere_api_key to '{hf_api_key}'"))
        .execute(&mut *tx)
        .await
        .unwrap();

    common::init_embedding_svc_url(&conn).await;
    // initialize a job
    let result = sqlx::query(&format!(
        "SELECT vectorize.table(
        job_name => '{job_name}',
        \"table\" => '{test_table_name}',
        primary_key => 'product_id',
        columns => ARRAY['product_name'],
        transformer => 'cohere/embed-multilingual-light-v3.0',
        schedule => 'realtime'
    );"
    ))
    .execute(&mut *tx)
    .await;
    tx.commit().await.unwrap();
    assert!(result.is_ok());

    let search_results: Vec<common::SearchJSON> =
        util::common::search_with_retry(&conn, "mobile devices", &job_name, 10, 2, 3, None)
            .await
            .unwrap();
    assert_eq!(search_results.len(), 3);
}
