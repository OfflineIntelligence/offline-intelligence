#ifndef OFFLINE_INTELLIGENCE_H
#define OFFLINE_INTELLIGENCE_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>
#include <stddef.h>

// Opaque handle for the OfflineIntelligence instance
typedef struct OfflineIntelligenceHandle OfflineIntelligenceHandle;

// Message structure
typedef struct {
    const char* role;
    const char* content;
} Message;

// Result structure for optimization
typedef struct {
    const Message* optimized_messages;
    int32_t original_count;
    int32_t optimized_count;
    float compression_ratio;
} OptimizationResult;

// Result structure for search
typedef struct {
    int32_t total;
    const char* search_type;
} SearchResult;

// Create a new OfflineIntelligence instance
OfflineIntelligenceHandle* offline_intelligence_new(void);

// Free an OfflineIntelligence instance
void offline_intelligence_free(OfflineIntelligenceHandle* handle);

// Optimize conversation context
OptimizationResult offline_intelligence_optimize_context(
    OfflineIntelligenceHandle* handle,
    const char* session_id,
    const Message* messages,
    int32_t message_count,
    const char* user_query
);

// Search memory
SearchResult offline_intelligence_search(
    OfflineIntelligenceHandle* handle,
    const char* query,
    const char* session_id,
    int32_t limit
);

// Generate title for conversation
char* offline_intelligence_generate_title(
    OfflineIntelligenceHandle* handle,
    const Message* messages,
    int32_t message_count
);

// Free a C string allocated by the library
void offline_intelligence_free_string(char* s);

#ifdef __cplusplus
}
#endif

#endif // OFFLINE_INTELLIGENCE_H