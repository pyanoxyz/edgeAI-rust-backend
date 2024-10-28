
use usearch::{Index, IndexOptions, MetricKind, ScalarKind, new_index};
use log::{info, error};
use crate::parser::parse_code::ChunkWithCompressedData;
use std::fs;


// let index: Index = new_index(&options).unwrap();

// assert!(index.reserve(10).is_ok());
// assert!(index.capacity() >= 10);
// assert!(index.connectivity() != 0);
// assert_eq!(index.dimensions(), 3);
// assert_eq!(index.size(), 0);

// let first: [f32; 3] = [0.2, 0.1, 0.2];
// let second: [f32; 3] = [0.2, 0.1, 0.2];

// assert!(index.add(42, &first).is_ok());
// assert!(index.add(43, &second).is_ok());
// assert_eq!(index.size(), 2);

// // Read back the tags
// let results = index.search(&first, 10).unwrap();
// assert_eq!(results.keys.len(), 2);


pub fn add_to_index(session_id: &str, chunks_with_data: Vec<ChunkWithCompressedData>) {
    // Load or create the index
    let index = load_or_create_index(session_id);

    // Iterate over the chunks and add each embedding to the index
    for chunk_with_data in chunks_with_data {
        match index.add(chunk_with_data.chunk_id, &chunk_with_data.embeddings) {
            Ok(_) => info!("Added chunk {} to index", chunk_with_data.chunk_id),
            Err(err) => error!(
                "Failed to add embeddings of length {} for chunk ID {}: {:?}",
                chunk_with_data.embeddings.len(),
                chunk_with_data.chunk_id,
                err
            ),
        };
    }

    // Save the index after adding all the embeddings
    if let Err(err) = save_index(&index, session_id) {
        error!("Failed to save the index for session {}: {:?}", session_id, err);
    } else {
        info!("Index successfully saved for session: {}", session_id);
    }
}

fn load_or_create_index(session_id: &str) -> Index {
    let options = IndexOptions {
        dimensions: 384, // necessary for most metric kinds, should match the dimension of embeddings
        metric: MetricKind::Cos, // or ::L2sq, ::Cos ...
        quantization: ScalarKind::F32, // or ::F32, ::F16, ::I8, ::B1x8 ...
        connectivity: 0,
        expansion_add: 0,
        expansion_search: 0,
        multi: false,
    };

    let index: Index = new_index(&options).unwrap();

    let home_directory = dirs::home_dir().unwrap();
    let root_pyano_dir = home_directory.join(".pyano");
    let pyano_data_dir = root_pyano_dir.join("indexes");

    if !pyano_data_dir.exists() {
        fs::create_dir_all(&pyano_data_dir).unwrap();
    }

    let index_name = format!("{}.usearch", session_id);
    let index_path = pyano_data_dir.join(index_name);
    let index_path_str = index_path.display().to_string();

    match index.load(&index_path_str) {
        Ok(_) => {
            info!("Loaded existing index for session: {}", session_id);
        }
        Err(err) => {
            info!("Index load failed for session: {} with error {}", session_id, err);
        }
    };
    index.reserve(10000000);
    index
    
}

fn save_index(index: &Index, session_id: &str) -> Result<(), String> {
    let home_directory = dirs::home_dir().unwrap();
    let root_pyano_dir = home_directory.join(".pyano");
    let pyano_data_dir = root_pyano_dir.join("indexes");

    let index_name = format!("{}.usearch", session_id);
    let index_path = pyano_data_dir.join(index_name);
    let index_path_str = index_path.display().to_string();

    match index.save(&index_path_str) {
        Ok(_) => {
            info!("Index successfully saved for session: {}", session_id);
            Ok(())
        }
        Err(err) => Err(format!(
            "Failed to save the index for session {}: {:?}",
            session_id, err
        )),
    }
}

pub fn search_index(session_id: &str, query_embedding: Vec<f32>) ->  Vec<u64>{
    // Load the index
    let index = load_or_create_index(session_id);
    let mut result_vec: Vec<u64> = Vec::new();

    // Perform the search on the index with the query embedding
    match index.search(&query_embedding, 10) {
        Ok(results) => {
            info!("Found {:?} results for session: {}", results, session_id);
            for (i, result) in results.keys.into_iter().enumerate() {
                info!("Result {}: {:?}", i + 1, result);
                result_vec.push(result);
            }
        },
        Err(err) => {
            error!(
                "Search failed for session: {} with error: {:?}",
                session_id, err
            );
        }
    }
    result_vec
}


pub fn remove_from_index(session_id: &str, chunk_ids: Vec<u64>) {
    // Load or create the index
    let index = load_or_create_index(session_id);

    // Iterate over the chunk_ids and remove each one from the index
    for chunk_id in chunk_ids {
        match index.remove(chunk_id) {
            Ok(_) => info!("Removed chunk {} from index", chunk_id),
            Err(err) => error!(
                "Failed to remove chunk ID {} from index: {:?}",
                chunk_id,
                err
            ),
        };
    }

    // Save the index after removing the chunks
    if let Err(err) = save_index(&index, session_id) {
        error!(
            "Failed to save the index after removal for session {}: {:?}",
            session_id, err
        );
    } else {
        info!("Index successfully updated and saved after removal for session: {}", session_id);
    }
}