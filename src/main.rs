use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};




// #[actix_web::main]
// async fn main() -> std::io::Result<()> {

//     let server_port = std::env::var("PORT")
//         .unwrap_or_else(|_| String::from("8080"))
//         .parse::<u16>()
//         .unwrap_or(8080);

//     HttpServer::new(move || {
//         let cors = Cors::permissive();
//         App::new()
//             .app_data(make_json_config())
//             .app_data(web::Data::new(custom_metrics.clone()))
//             .wrap(cors)
//             .wrap(middleware::Logger::default())
//             .wrap(RequestTracing::new())
//             .wrap(generic_http_metrics.clone())
//             .route(
//                 "/metrics",
//                 web::get().to(PrometheusMetricsHandler::new(exporter.clone())),
//             )
//             .service(
//                 web::scope("/api")
//                     .wrap(ClerkMiddleware::new(clerk_config.clone(), None))
//                     .service(
//                         web::scope("/v1/orgs")
//                             .service(instance::get_all)
//                             .service(instance::get_instance)
//                             .service(instance::create_instance)
//                             .service(instance::put_instance)
//                             .service(instance::patch_instance)
//                             .service(instance::delete_instance)
//                             .service(instance::instance_event)
//                             .service(instance::get_schema)
//                             .service(instance::restore_instance),
//                     )
//                     .service(
//                         web::scope("/v1/stacks")
//                             .service(stack::get_entity)
//                             .service(stack::get_all_entities),
//                     ),
//             )
//             .service(
//                 web::scope("/auth")
//                     .service(auth)
//                     .app_data(web::Data::new(Authorizer::new(Clerk::new(
//                         clerk_config.clone(),
//                     )))),
//             )
//             .app_data(web::Data::new(Authorizer::new(Clerk::new(clerk_config))))
//             .app_data(web::Data::new(dbclient.clone()))
//             .app_data(web::Data::new(queue.clone()))
//             .app_data(web::Data::new(cfg.clone()))
//             .service(web::scope("/").service(root::ok))
//             .service(web::scope("/health").service(ready).service(lively))
//             .service(SwaggerUi::new("/swagger-ui/{_:.*}").urls(vec![(
//                 Url::with_primary("tembo-cloud-platform", "/api-docs/openapi.json", true),
//                 v1doc.clone(),
//             )]))
//             .service(Redoc::with_url("/redoc", redoc_v1doc.clone()))
//     })
//     .workers(8)
//     // TCP keep-alive should be greater than idle-timeout of ALB
//     // default ALB timeout is 60 seconds.
//     // https://docs.aws.amazon.com/elasticloadbalancing/latest/classic/config-idle-timeout.html
//     .keep_alive(Duration::from_secs(75))
//     .bind(("0.0.0.0", server_port))?
//     .run()
//     .await
// }

use rust_bert::pipelines::sentence_embeddings::SentenceEmbeddingsBuilder;

fn main() -> anyhow::Result<()> {
    // Set-up sentence embeddings model
    let model = SentenceEmbeddingsBuilder::local("/all-MiniLM-L12-v2")
        .with_device(tch::Device::cuda_if_available())
        .create_model()?;

    // Define input
    let sentences = ["this is an example sentence", "each sentence is converted"];

    // Generate Embeddings
    let embeddings = model.encode(&sentences)?;
    println!("{embeddings:?}");
    Ok(())
}