//! Integration tests for the KV cache lifecycle with llama-server
//!
//! These tests validate that the KV cache system properly manages the lifecycle:
//! 1. Cache accumulation during conversations
//! 2. Memory threshold detection and clearing
//! 3. Data persistence before clearing
//! 4. Context restoration and continuity

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache_management::{KVCacheManager, KVCacheConfig, KVEntry};
    use crate::memory_db::MemoryDatabase;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_kv_cache_lifecycle() {
        // Initialize database
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let database = Arc::new(MemoryDatabase::new(&db_path).await.unwrap());

        // Create cache manager with test config
        let config = KVCacheConfig {
            enabled: true,
            retrieval_enabled: true,
            clear_after_conversations: 3, // Lower for testing
            memory_threshold_percent: 0.1, // Low threshold for testing
            bridge_enabled: true,
            max_cache_entries: 100,
            min_importance_to_preserve: 0.5,
            generate_cache_embeddings: true,
            retrieval_strategy: crate::cache_management::RetrievalStrategy::KeywordThenSemantic,
            preserve_system_prompts: true,
            preserve_code_entries: true,
            snapshot_strategy: crate::cache_management::SnapshotStrategy::Incremental {
                interval_conversations: 2,
                max_snapshots: 3,
            },
        };

        let mut cache_manager = KVCacheManager::new(config, database.clone()).unwrap();

        // Simulate multiple conversations
        let session_id = "test_session_1";
        
        // Create test KV entries that simulate attention cache data
        let test_entries: Vec<KVEntry> = (0..50).map(|i| KVEntry {
            key_hash: format!("test_key_{}", i),
            key_data: Some(vec![i as u8; 10]),
            value_data: vec![i as u8; 20],
            key_type: if i % 2 == 0 { "attention_key".to_string() } else { "attention_value".to_string() },
            layer_index: (i % 8) as i32, // 8 layers
            head_index: Some((i % 4) as i32), // 4 heads
            importance_score: if i < 25 { 0.8 } else { 0.3 }, // First half is more important
            access_count: 1,
            last_accessed: chrono::Utc::now(),
            token_positions: Some(vec![i as u32]),
            embedding: None,
            size_bytes: 30, // 10 + 20 bytes
            is_persistent: i < 10, // First 10 marked as persistent
        }).collect();

        // Process multiple conversations to trigger clearing
        for conv_num in 1..=5 {
            let result = cache_manager.process_conversation(
                session_id,
                &[], // No messages for this test
                &test_entries,
                test_entries.len() * 30, // Total size
                1000, // Max cache size for testing
            ).await.unwrap();

            println!("Conversation {}: Clear triggered: {}, Retrieve triggered: {}", 
                conv_num, result.should_clear_cache, result.should_retrieve);

            if result.should_clear_cache {
                assert!(result.clear_result.is_some());
                let clear_result = result.clear_result.unwrap();
                println!("Cleared {} entries, preserved {} entries", 
                    clear_result.entries_cleared, clear_result.entries_to_keep.len());
                
                // Verify that important entries were preserved
                assert!(clear_result.entries_to_keep.len() > 0);
            }
        }

        // Verify that snapshots were created
        let stats = cache_manager.export_statistics();
        println!("Total clears: {}, Total retrievals: {}, Entries preserved: {}", 
            stats.total_clears, stats.total_retrievals, stats.entries_preserved);
        
        assert!(stats.total_clears > 0);
        assert!(stats.entries_preserved > 0);
    }

    #[tokio::test]
    async fn test_memory_based_clearing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test2.db");
        let database = Arc::new(MemoryDatabase::new(&db_path).await.unwrap());

        let config = KVCacheConfig {
            enabled: true,
            retrieval_enabled: true,
            clear_after_conversations: 100, // High to avoid conversation-based clearing
            memory_threshold_percent: 0.05, // Very low to trigger memory-based clearing
            bridge_enabled: true,
            max_cache_entries: 50,
            min_importance_to_preserve: 0.3,
            generate_cache_embeddings: true,
            retrieval_strategy: crate::cache_management::RetrievalStrategy::KeywordOnly,
            preserve_system_prompts: true,
            preserve_code_entries: true,
            snapshot_strategy: crate::cache_management::SnapshotStrategy::None,
        };

        let mut cache_manager = KVCacheManager::new(config, database.clone()).unwrap();

        let session_id = "test_session_2";
        
        // Create entries that will exceed memory threshold
        let large_entries: Vec<KVEntry> = (0..20).map(|i| KVEntry {
            key_hash: format!("large_key_{}", i),
            key_data: Some(vec![i as u8; 100]), // Larger entries
            value_data: vec![i as u8; 200],    // Larger entries
            key_type: "attention_value".to_string(),
            layer_index: (i % 4) as i32,
            head_index: Some((i % 2) as i32),
            importance_score: if i % 3 == 0 { 0.9 } else { 0.2 }, // Every third is important
            access_count: 1,
            last_accessed: chrono::Utc::now(),
            token_positions: Some(vec![i as u32, (i + 1) as u32]),
            embedding: None,
            size_bytes: 300, // 100 + 200 bytes
            is_persistent: i % 5 == 0, // Every fifth is persistent
        }).collect();

        // Process conversation with large entries
        let result = cache_manager.process_conversation(
            session_id,
            &[],
            &large_entries,
            large_entries.len() * 300, // Large total size
            1000, // Small max size to trigger memory clearing
        ).await.unwrap();

        // Should trigger memory-based clearing
        assert!(result.should_clear_cache);
        if let Some(clear_result) = result.clear_result {
            // Should preserve important entries
            assert!(clear_result.entries_to_keep.len() > 0);
            println!("Memory-based clear preserved {} important entries out of {}", 
                clear_result.entries_to_keep.len(), large_entries.len());
        }
    }

    #[tokio::test]
    async fn test_context_continuity_via_bridge() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test3.db");
        let database = Arc::new(MemoryDatabase::new(&db_path).await.unwrap());

        let config = KVCacheConfig {
            enabled: true,
            retrieval_enabled: true,
            clear_after_conversations: 2,
            memory_threshold_percent: 0.8,
            bridge_enabled: true, // Enable bridge messages
            max_cache_entries: 50,
            min_importance_to_preserve: 0.4,
            generate_cache_embeddings: true,
            retrieval_strategy: crate::cache_management::RetrievalStrategy::KeywordThenSemantic,
            preserve_system_prompts: true,
            preserve_code_entries: true,
            snapshot_strategy: crate::cache_management::SnapshotStrategy::Full { interval_conversations: 1 },
        };

        let mut cache_manager = KVCacheManager::new(config, database.clone()).unwrap();

        let session_id = "test_session_3";
        
        // Create mixed importance entries
        let mixed_entries: Vec<KVEntry> = (0..10).map(|i| KVEntry {
            key_hash: format!("mixed_key_{}", i),
            key_data: Some(vec![i as u8; 10]),
            value_data: vec![i as u8; 20],
            key_type: "attention_key".to_string(),
            layer_index: 0,
            head_index: Some(i as i32),
            importance_score: if i < 5 { 0.9 } else { 0.1 }, // First 5 are important
            access_count: 1,
            last_accessed: chrono::Utc::now(),
            token_positions: Some(vec![i as u32]),
            embedding: None,
            size_bytes: 30,
            is_persistent: false,
        }).collect();

        // Process conversation that will trigger clearing
        let result = cache_manager.process_conversation(
            session_id,
            &[],
            &mixed_entries,
            mixed_entries.len() * 30,
            2000, // Larger max size
        ).await.unwrap();

        // Should clear and generate bridge messages
        assert!(result.should_clear_cache);
        assert!(!result.bridge_messages.is_empty());
        
        if let Some(clear_result) = result.clear_result {
            // Should preserve high-importance entries
            assert_eq!(clear_result.entries_to_keep.len(), 5); // First 5 are high importance
            println!("Bridge message: {}", clear_result.bridge_message);
        }
    }
}