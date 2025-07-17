use anyhow::Context;
use s3_vectors::{
    batch_put_vectors, create_bucket_and_index, 
    S3VectorsClient, Vector, DistanceMetric, QueryVector, QueryVectorsRequest,
    GetVectorsRequest, ListVectorsRequest, DeleteVectorsRequest,
};
use serde_json::json;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create client from environment variables or use default
    let client = match S3VectorsClient::from_env() {
        Ok(client) => {
            tracing::info!("Using credentials from environment");
            client
        }
        Err(_) => {
            tracing::warn!("No credentials found in environment, using default client");
            S3VectorsClient::new("us-east-1")
        }
    };

    // Define test bucket and index names
    let bucket_name = "test-vectors";
    let index_name = "products";
    let dimensions = 128;

    // Create bucket and index
    tracing::info!("Creating vector bucket and index...");
    let (bucket, index) = create_bucket_and_index(
        &client, 
        bucket_name, 
        index_name, 
        dimensions, 
        DistanceMetric::Cosine
    )
    .await
    .context("failed to create bucket and index")?;
    
    tracing::info!(
        "Created bucket: {} (status: {:?})",
        bucket.bucket_name,
        bucket.status
    );
    tracing::info!(
        "Created index: {} (status: {:?}, dimensions: {})",
        index.index_name,
        index.status,
        index.vector_dimensions
    );

    // Create sample vectors
    let vectors = vec![
        Vector {
            key: "product-1".to_string(),
            vector: vec![0.1; dimensions as usize],
            metadata: Some(json!({
                "name": "Laptop Pro 2024",
                "category": "electronics",
                "subcategory": "computers",
                "price": 1299.99,
                "brand": "TechCorp",
                "in_stock": true
            })),
        },
        Vector {
            key: "product-2".to_string(),
            vector: vec![0.2; dimensions as usize],
            metadata: Some(json!({
                "name": "Smartphone X",
                "category": "electronics",
                "subcategory": "phones",
                "price": 799.99,
                "brand": "PhoneCo",
                "in_stock": true
            })),
        },
        Vector {
            key: "product-3".to_string(),
            vector: vec![0.3; dimensions as usize],
            metadata: Some(json!({
                "name": "Wireless Headphones",
                "category": "electronics",
                "subcategory": "audio",
                "price": 249.99,
                "brand": "AudioTech",
                "in_stock": false
            })),
        },
        Vector {
            key: "product-4".to_string(),
            vector: vec![0.15; dimensions as usize],
            metadata: Some(json!({
                "name": "Tablet Ultra",
                "category": "electronics",
                "subcategory": "tablets",
                "price": 599.99,
                "brand": "TechCorp",
                "in_stock": true
            })),
        },
    ];

    // Put vectors in batches
    tracing::info!("Inserting {} vectors...", vectors.len());
    batch_put_vectors(&client, bucket_name, index_name, vectors, dimensions)
        .await
        .context("failed to put vectors")?;

    // List vectors
    tracing::info!("Listing vectors...");
    let list_request = ListVectorsRequest {
        bucket_name: bucket_name.to_string(),
        index_name: index_name.to_string(),
        max_results: Some(10),
        next_token: None,
    };
    
    let list_response = client
        .list_vectors(list_request)
        .await
        .context("failed to list vectors")?;
    
    tracing::info!("Found {} vector keys", list_response.keys.len());
    for key in &list_response.keys {
        tracing::info!("  - {}", key);
    }

    // Get specific vectors
    tracing::info!("Getting specific vectors...");
    let get_request = GetVectorsRequest {
        bucket_name: bucket_name.to_string(),
        index_name: index_name.to_string(),
        keys: vec!["product-1".to_string(), "product-3".to_string()],
        return_vector: true,
        return_metadata: true,
    };
    
    let get_response = client
        .get_vectors(get_request)
        .await
        .context("failed to get vectors")?;
    
    for vector in &get_response.vectors {
        tracing::info!(
            "Retrieved vector: key={}, has_vector={}, metadata={:?}",
            vector.key,
            vector.vector.is_some(),
            vector.metadata
        );
    }

    // Query vectors with different filters
    tracing::info!("Querying vectors...");
    
    // Query 1: Find similar products in stock
    let query_request1 = QueryVectorsRequest {
        bucket_name: bucket_name.to_string(),
        index_name: index_name.to_string(),
        query_vector: QueryVector {
            float32: vec![0.12; dimensions as usize], // Similar to product-1
        },
        top_k: 3,
        filter: Some(json!({
            "in_stock": {"$eq": true}
        })),
        return_metadata: true,
        return_distance: true,
    };

    let results1 = client
        .query_vectors(query_request1)
        .await
        .context("failed to query vectors")?;

    tracing::info!("Query 1 - Products in stock:");
    for result in &results1.vectors {
        tracing::info!(
            "  key: {}, distance: {:.4}, metadata: {:?}",
            result.key,
            result.distance.unwrap_or(0.0),
            result.metadata
        );
    }

    // Query 2: Find products by brand
    let query_request2 = QueryVectorsRequest {
        bucket_name: bucket_name.to_string(),
        index_name: index_name.to_string(),
        query_vector: QueryVector {
            float32: vec![0.25; dimensions as usize],
        },
        top_k: 5,
        filter: Some(json!({
            "brand": {"$eq": "TechCorp"}
        })),
        return_metadata: true,
        return_distance: true,
    };

    let results2 = client
        .query_vectors(query_request2)
        .await
        .context("failed to query vectors")?;

    tracing::info!("Query 2 - TechCorp products:");
    for result in &results2.vectors {
        tracing::info!(
            "  key: {}, distance: {:.4}, name: {}",
            result.key,
            result.distance.unwrap_or(0.0),
            result.metadata.as_ref()
                .and_then(|m| m.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("Unknown")
        );
    }

    // Query 3: Price range filter
    let query_request3 = QueryVectorsRequest {
        bucket_name: bucket_name.to_string(),
        index_name: index_name.to_string(),
        query_vector: QueryVector {
            float32: vec![0.18; dimensions as usize],
        },
        top_k: 10,
        filter: Some(json!({
            "$and": [
                {"price": {"$gte": 500}},
                {"price": {"$lte": 1000}}
            ]
        })),
        return_metadata: true,
        return_distance: true,
    };

    let results3 = client
        .query_vectors(query_request3)
        .await
        .context("failed to query vectors")?;

    tracing::info!("Query 3 - Products $500-$1000:");
    for result in &results3.vectors {
        tracing::info!(
            "  key: {}, distance: {:.4}, price: ${}",
            result.key,
            result.distance.unwrap_or(0.0),
            result.metadata.as_ref()
                .and_then(|m| m.get("price"))
                .and_then(|p| p.as_f64())
                .unwrap_or(0.0)
        );
    }

    // List buckets
    tracing::info!("Listing vector buckets...");
    let buckets_response = client
        .list_vector_buckets(Some(10), None)
        .await
        .context("failed to list buckets")?;
    
    tracing::info!("Found {} vector buckets", buckets_response.buckets.len());
    for bucket in &buckets_response.buckets {
        tracing::info!(
            "  - {} (region: {}, status: {:?})",
            bucket.bucket_name,
            bucket.region,
            bucket.status
        );
    }

    // List indexes in bucket
    tracing::info!("Listing indexes in bucket...");
    let indexes_response = client
        .list_indexes(bucket_name, Some(10), None)
        .await
        .context("failed to list indexes")?;
    
    tracing::info!("Found {} indexes", indexes_response.indexes.len());
    for index in &indexes_response.indexes {
        tracing::info!(
            "  - {} (dimensions: {}, metric: {:?}, vectors: {:?})",
            index.index_name,
            index.vector_dimensions,
            index.distance_metric,
            index.vector_count
        );
    }

    // Delete some vectors
    tracing::info!("Deleting vector product-3...");
    let delete_request = DeleteVectorsRequest {
        bucket_name: bucket_name.to_string(),
        index_name: index_name.to_string(),
        keys: vec!["product-3".to_string()],
    };
    
    client
        .delete_vectors(delete_request)
        .await
        .context("failed to delete vectors")?;
    
    tracing::info!("Vector deleted successfully");

    // Cleanup (commented out to preserve data for testing)
    // tracing::info!("Cleaning up...");
    // client
    //     .delete_index(bucket_name, index_name)
    //     .await
    //     .context("failed to delete index")?;
    // 
    // client
    //     .delete_vector_bucket(bucket_name)
    //     .await
    //     .context("failed to delete bucket")?;
    
    tracing::info!("Demo completed successfully!");
    tracing::info!("Note: Bucket and index were left intact for further testing.");
    tracing::info!("To clean up, uncomment the cleanup section in main.rs");

    Ok(())
}