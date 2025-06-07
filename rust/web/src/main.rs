
#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use web::app::*;
    use web::db::create_pool;
    use web::auth::session::create_session_layer;


    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    // Create database connection pool
    let pool = create_pool().await.expect("Failed to create database pool");
    
    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    let app = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            let pool = pool.clone();
            move || {
                provide_context(pool.clone());
                shell(leptos_options.clone())
            }
        })
        .fallback(leptos_axum::file_and_error_handler({
            let leptos_options = leptos_options.clone();
            move |_| shell(leptos_options.clone())
        }))
        .layer(create_session_layer())
        .with_state(leptos_options);

    // run our app with hyper
    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
