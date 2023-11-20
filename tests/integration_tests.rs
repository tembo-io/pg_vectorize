use rand::Rng;
mod util;

use util::common;

// Integration tests are ignored by default
#[ignore]
#[tokio::test]
async fn test_table() {
    let conn = common::init_database().await;
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
        transformer => 'all_MiniLM_L12_v2'
    );"
    ))
    .execute(&conn)
    .await
    .expect("failed to init job");

    // no jobs in queue at beginning
    let rowcount = common::row_count(&format!("pgmq.q_v_all_MiniLM_L12_v2"), &conn).await;
    assert_eq!(rowcount, 0);

    // manually trigger a job
    let _ = sqlx::query(&format!("SELECT vectorize.job_execute('{job_name}');"))
        .execute(&conn)
        .await
        .expect("failed to select from test_table");

    // sleep 500ms
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    // should 1 job in the queue
    let rowcount = common::row_count(&format!("pgmq.q_v_all_MiniLM_L12_v2"), &conn).await;
    assert_eq!(rowcount, 1);
}
