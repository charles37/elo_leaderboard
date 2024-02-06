use tokio::*;
use reqwest;
use serde_json;
use dotenv::dotenv;
use strsim::jaro_winkler;
use csv;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;



// src/main.rs


//CREATE TABLE concepts (
//    id SERIAL PRIMARY KEY,
//    title TEXT NOT NULL,
//    link TEXT NOT NULL,
//    category TEXT NOT NULL,
//    elo_score INT DEFAULT 1200
//);

#[derive(sqlx::FromRow, Debug, serde::Serialize, serde::Deserialize)]
struct Concept {
    id: i32,
    title: String,
    link: String,
    category: String,
    elo_score: Option<i32>
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let db_pool = create_db_pool().await.unwrap();



    //let categories = [
    //    "Culture", "Geography", "Health", "History", "Human activities", 
    //    "Mathematics", "Nature", "People", "Philosophy", "Religion", 
    //    "Society", "Technology", "Politics"
    //];
    //


    //for category in &categories {
    //    match fetch_top_articles_from_category(category, 10).await {
    //        Ok(articles) => {
    //            for article in &articles {
    //                let link = format!("https://en.wikipedia.org/wiki/{}", article.replace(" ", "_"));
    //                match insert_concept(&db_pool, article, &link, category).await {
    //                    Ok(_) => {},
    //                    Err(e) => {
    //                        eprintln!("Error inserting article {}: {}", article, e);
    //                    }
    //                }
    //            }
    //        },
    //        Err(e) => {
    //            eprintln!("Error fetching top articles from category {}: {}", category, e);
    //        }
    //    }
    //}


    //get all concepts from db
   
    // write the concepts to a csv file
    //let mut wtr = csv::Writer::from_path("concepts.csv").unwrap();
    //for concept in &concepts {
    //    wtr.serialize(concept).unwrap();
    //}

    // run the full suite, matching concepts against each other and updating their elo scores


    reset_all_to_1200(&db_pool).await.unwrap();

    let mut concepts = get_all(&db_pool).await.unwrap();

    //print the concepts
    for concept in &concepts {
        println!("{:?}", concept);
    }

    let n = concepts.len();

    for round in 0..n {
        for i in (0..n).step_by(2) {
            let j = (i + 1) % n;
            let concept_a = &concepts[i];
            let concept_b = &concepts[j];

            println!("Matching {} and {}, which are ids {} and {} in round {}", concept_a.title, concept_b.title, i, j, round);
            match match_and_update_elo_with_id(&db_pool, concept_a.id.clone(), concept_b.id.clone()).await {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("Error matching {} and {}: {}", concept_a.title, concept_b.title, e);
                }
            }
        }

        // Rotate the concepts for the next round
        let first = concepts.remove(0);
        concepts.push(first);
    }


    //for i in 0..concepts.len() {
    //    for j in i+1..concepts.len() {
    //        if i != j {
    //            println!("Matching {} and {}, which are ids {} and {}", concepts[i].title, concepts[j].title, i, j);
    //            match match_and_update_elo_with_id(&db_pool, concepts[i].id.clone(), concepts[j].id.clone()).await {
    //                Ok(_) => {},
    //                Err(e) => {
    //                    eprintln!("Error matching {} and {}: {}", concepts[i].title, concepts[j].title, e);
    //                }
    //            }
    //        }
    //    }
    //}
    
    // write the concepts to a csv file
    let mut wtr = csv::Writer::from_path("conceptsb.csv").unwrap();
    for concept in &concepts {
        wtr.serialize(concept).unwrap();
    }


    // get the top 20 concepts
    //let top_concepts = get_top(&db_pool, 20).await.unwrap();
    //println!("{:?}", top_concepts);
}


const WIKIPEDIA_API_ENDPOINT: &str = "https://en.wikipedia.org/w/api.php";

async fn fetch_articles_from_category(category: &str, limit: usize) -> Result<Vec<String>, reqwest::Error> {
    let client = reqwest::Client::new();
    let mut articles = Vec::new();
    let mut continue_param = String::new();

    while articles.len() < limit {
        let response = client.get(WIKIPEDIA_API_ENDPOINT)
            .query(&[
                ("action", "query"),
                ("format", "json"),
                ("list", "categorymembers"),
                ("cmtitle", &format!("Category:{}", category)),
                ("cmlimit", "500"),
                ("cmcontinue", &continue_param),
                ("cmtype", "page") // Only fetch pages, not subcategories or files
            ])
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;
        if let Some(members) = json["query"]["categorymembers"].as_array() {
            for member in members {
                if let Some(title) = member["title"].as_str() {
                    articles.push(title.to_string());
                }
            }
        }

        

        // Check if there's more data to fetch
        if let Some(continue_data) = json["continue"]["cmcontinue"].as_str() {
            continue_param = continue_data.to_string();
        } else {
            break;
        }
    }

    Ok(articles)
}

async fn fetch_top_articles_from_category(category: &str, limit: usize) -> Result<Vec<String>, reqwest::Error> {
    let articles = fetch_articles_from_category(category, 500).await?; // Fetch 500 articles as an example
    let mut articles_with_pageviews = Vec::new();

    for article in &articles {
        match fetch_pageviews_for_article(article).await {
            Ok(pageviews) => {
                articles_with_pageviews.push((article.clone(), pageviews));
            },
            Err(e) => {
                eprintln!("Error fetching pageviews for article {}: {}", article, e);
            }
        }
    }

    // Sort articles by pageviews and take the top `limit`
    articles_with_pageviews.sort_by(|a, b| b.1.cmp(&a.1));
    let top_articles: Vec<String> = articles_with_pageviews.into_iter().map(|(article, _)| article).take(limit).collect();

    Ok(top_articles)
}


async fn fetch_pageviews_for_article(title: &str) -> Result<i64, reqwest::Error> {
    let client = reqwest::Client::new();
    let endpoint = format!("https://wikimedia.org/api/rest_v1/metrics/pageviews/per-article/en.wikipedia/all-access/all-agents/{}/daily/20230101/20230131", title);

    let response = client.get(&endpoint).send().await?;
    let json: serde_json::Value = response.json().await?;

    let mut total_pageviews = 0;
    if let Some(items) = json["items"].as_array() {
        for item in items {
            if let Some(views) = item["views"].as_i64() {
                total_pageviews += views;
            }
        }
    }

    Ok(total_pageviews)
}

use sqlx::postgres::PgPool;

async fn create_db_pool() -> Result<PgPool, sqlx::Error> {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await?;
    Ok(pool)
}

async fn insert_concept(pool: &PgPool, title: &str, link: &str, category: &str) -> Result<i32, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        INSERT INTO concepts (title, link, category)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
        title, link, category
    )
    .fetch_one(pool)
    .await?;

    dbg!("Inserted concept {} with id {}", title, result.id);

    Ok(result.id)
}

async fn update_elo_score(pool: &PgPool, id: i32, new_elo: i32) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE concepts
        SET elo_score = $1
        WHERE id = $2
        "#,
        new_elo, id
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn get_concept_by_id(pool: &PgPool, id: i32) -> Result<Concept, sqlx::Error> {
    let concept = sqlx::query_as!(Concept, "SELECT * FROM concepts WHERE id = $1", id)
        .fetch_one(pool)
        .await?;

    Ok(concept)
}

async fn get_all(pool: &PgPool) -> Result<Vec<Concept>, sqlx::Error> {
    let concepts = sqlx::query_as!(Concept, "SELECT * FROM concepts")
        .fetch_all(pool)
        .await?;

    Ok(concepts)
}

fn compute_elo(rating1: i32, rating2: i32, outcome: f64) -> (i32, i32) {
    let k = 32.0;
    let expected_outcome1 = 1.0 / (1.0 + 10.0f64.powf((rating2 - rating1) as f64 / 400.0));
    let expected_outcome2 = 1.0 - expected_outcome1;

    let new_rating1 = (rating1 as f64 + k * (outcome - expected_outcome1)).round() as i32;
    let new_rating2 = (rating2 as f64 + k * (1.0 - outcome - expected_outcome2)).round() as i32;

    (new_rating1, new_rating2)
}

use async_openai::{types::CreateCompletionRequestArgs, Client};

async fn match_and_update_elo(pool: &PgPool, concept1: &mut Concept, concept2: &mut Concept) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let max_tokens = std::cmp::max(concept1.title.len(), concept2.title.len()) as u16 + 10;

    let prompt = format!("Which is better: {} or {}? you must output only one of the choices and no other words", concept1.title, concept2.title);
    let request = CreateCompletionRequestArgs::default()
        .model("text-davinci-003")
        .prompt(&prompt)
        .max_tokens(max_tokens)
        .build()?;

    let response = client.completions().create(request).await?;

    let response_text = response.choices[0].text.trim();
    let similarity1 = jaro_winkler(&response_text, &concept1.title);
    let similarity2 = jaro_winkler(&response_text, &concept2.title);

    let outcome = if similarity1 > similarity2 && similarity1 > 0.85 {
        1.0
    } else if similarity2 > similarity1 && similarity2 > 0.85 {
        0.0
    } else {
        dbg!(response_text); 
        0.5
    };

    let (new_rating1, new_rating2) = compute_elo(concept1.elo_score.unwrap_or(1200), concept2.elo_score.unwrap_or(1200), outcome);

    update_elo_score(pool, concept1.id, new_rating1).await?;
    update_elo_score(pool, concept2.id, new_rating2).await?;

    println!("{}: {} -> {}", concept1.title, concept1.elo_score.unwrap_or(1200), new_rating1);
    println!("{}: {} -> {}", concept2.title, concept2.elo_score.unwrap_or(1200), new_rating2);

    Ok(())
}

async fn match_and_update_elo_with_id(pool: &PgPool, id1: i32, id2: i32) -> Result<(), Box<dyn std::error::Error>> {
    let mut concept1 = get_concept_by_id(pool, id1).await?;
    let mut concept2 = get_concept_by_id(pool, id2).await?;

    match_and_update_elo(pool, &mut concept1, &mut concept2).await?;

    Ok(())
}


async fn get_top(pool: &PgPool, limit: i64) -> Result<Vec<Concept>, sqlx::Error> {
    let concepts = sqlx::query_as!(Concept, "SELECT * FROM concepts ORDER BY elo_score DESC LIMIT $1", limit)
        .fetch_all(pool)
        .await?;

    Ok(concepts)
}

async fn reset_all_to_1200(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE concepts
        SET elo_score = 1200
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[derive(Serialize)]
struct Payload {
    userId: &'static str,
    page: i32,
    pageLength: i32,
    reverseSort: bool,
    daily: bool,
    category: Option<String>
}

#[derive(Deserialize)]
struct ResponseItem {
    itemId: String,
    created: String,
    name: String,
    summary: String,
    image: String,
    wikipedia: String,
    categories: Vec<String>,
    score: Value,  // This type can be further refined if needed
}

const ENDPOINT: &str = "https://eloeverything.co/"; 

async fn fetch_leaderboard() -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();

    let payload = Payload {
        userId: "8XuAgqzIdzSS",
        page: 1,
        pageLength: 50,
        reverseSort: false,
        daily: false,
        category: None,
    };

    let response: Vec<ResponseItem> = client.post(ENDPOINT)
        .json(&payload)
        .send()
        .await?
        .json()
        .await?;

    for item in &response {
        println!("Name: {}, Summary: {}", item.name, item.summary);
    }

    Ok(())
}
