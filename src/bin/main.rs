use anyhow::Result;
use http::Response;
use recipe_analysis;
use recipe_api::connect_db;
use serde_derive::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, info, Level};
use warp::Filter;

#[derive(Deserialize, Serialize)]
struct RecipeFilter {
    tag: Option<String>,
    ingredients: Vec<String>,
    algorithm: String,
}

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("no global subscriber has been set");

    // POST /embed/
    let embed = warp::post()
        .and(warp::path("embed"))
        // Only accept bodies smaller than 16kb
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::json())
        .and_then(|filter: RecipeFilter| async {
            info!(
                "Embedding request
                Algorithm: {} 
                Tag: {}
                Ingredients: {}",
                &filter.algorithm,
                &filter.tag.as_ref().map_or("None", |tag| tag),
                &filter.ingredients.join(" ")
            );

            if let Ok(request_id) = new_request().await {
                let _embed_handle = tokio::spawn(async move {
                    if let Err(_) = embed(filter, request_id).await {
                        error!("Embedding failed for id: {}", request_id);
                    }
                });

                Ok(Response::builder().body(request_id.to_string()))
            } else {
                info!("Failed to process embedding request");
                Err(warp::reject::not_found())
            }
        });

    // GET /status/
    let status = warp::path!("status" / i32).and_then(|id| async move {
        info!("status request for id: {}", id);
        if let Ok(status) = get_status(id).await {
            let json = json!({ "status": status });
            Ok(Response::builder().body(json.to_string()))
        } else {
            Err(warp::reject::not_found())
        }
    });

    let status = warp::get().and(status);

    warp::serve(embed.or(status))
        .run(([127, 0, 0, 1], 8000))
        .await
}

async fn get_status(id: i32) -> Result<String> {
    let client = connect_db().await?;

    let statement = client
        .prepare("SELECT status FROM requests WHERE id = $1")
        .await?;

    let status = client.query(&statement, &[&id]).await?;

    Ok(status[0].try_get(0)?)
}

async fn new_request() -> Result<i32> {
    let client = connect_db().await?;

    let statement = client
        .prepare("INSERT INTO requests(status) VALUES('Selecting Recipes') RETURNING id")
        .await?;

    let request_id = client.query(&statement, &[]).await?;

    Ok(request_id[0].try_get(0)?)
}

async fn embed(filter: RecipeFilter, id: i32) -> Result<()> {
    let client = connect_db().await?;

    let recipes = recipe_analysis::recipe::pull_recipes(filter.tag.clone());
    let _ = client
        .query_opt(
            "UPDATE requests SET status = 'Building Co-occurence Matrix' WHERE id = $1",
            &[&id],
        )
        .await?;
    recipe_analysis::co_occurrence::make_coolist(recipes, filter.ingredients);
    let _ = client
        .query_opt(
            "UPDATE requests SET status = 'Embedding Relation' WHERE id = $1",
            &[&id],
        )
        .await?;
    let _ = client
        .query_opt(
            "UPDATE requests SET status = 'Complete' WHERE id = $1",
            &[&id],
        )
        .await?;
    info!("Made coolist for id: {}", id);
    Ok(())
}
