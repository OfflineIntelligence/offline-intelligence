#ifndef OFFLINE_INTELLIGENCE_CPP_H
#define OFFLINE_INTELLIGENCE_CPP_H

#include "offline_intelligence.h"
#include <string>
#include <vector>
#include <stdexcept>

namespace offline_intelligence {

class Message {
public:
    std::string role;
    std::string content;
    
    Message(const std::string& r, const std::string& c) 
        : role(r), content(c) {}
};

class OptimizationResult {
public:
    std::vector<Message> optimized_messages;
    int32_t original_count;
    int32_t optimized_count;
    float compression_ratio;
};

class SearchResult {
public:
    int32_t total;
    std::string search_type;
};

class OfflineIntelligence {
private:
    OfflineIntelligenceHandle* handle;

public:
    OfflineIntelligence() {
        handle = offline_intelligence_new();
        if (!handle) {
            throw std::runtime_error("Failed to create OfflineIntelligence instance");
        }
    }
    
    ~OfflineIntelligence() {
        if (handle) {
            offline_intelligence_free(handle);
        }
    }
    
    // Prevent copying
    OfflineIntelligence(const OfflineIntelligence&) = delete;
    OfflineIntelligence& operator=(const OfflineIntelligence&) = delete;
    
    // Allow moving
    OfflineIntelligence(OfflineIntelligence&& other) noexcept 
        : handle(other.handle) {
        other.handle = nullptr;
    }
    
    OfflineIntelligence& operator=(OfflineIntelligence&& other) noexcept {
        if (this != &other) {
            if (handle) {
                offline_intelligence_free(handle);
            }
            handle = other.handle;
            other.handle = nullptr;
        }
        return *this;
    }
    
    OptimizationResult optimize_context(
        const std::string& session_id,
        const std::vector<Message>& messages,
        const std::string& user_query = "") {
        
        // Convert C++ messages to C messages
        std::vector<::Message> c_messages;
        c_messages.reserve(messages.size());
        
        for (const auto& msg : messages) {
            ::Message c_msg;
            c_msg.role = msg.role.c_str();
            c_msg.content = msg.content.c_str();
            c_messages.push_back(c_msg);
        }
        
        auto result = offline_intelligence_optimize_context(
            handle,
            session_id.c_str(),
            c_messages.data(),
            static_cast<int32_t>(c_messages.size()),
            user_query.empty() ? nullptr : user_query.c_str()
        );
        
        OptimizationResult cpp_result;
        cpp_result.original_count = result.original_count;
        cpp_result.optimized_count = result.optimized_count;
        cpp_result.compression_ratio = result.compression_ratio;
        
        return cpp_result;
    }
    
    SearchResult search(
        const std::string& query,
        const std::string& session_id = "",
        int32_t limit = 10) {
        
        auto result = offline_intelligence_search(
            handle,
            query.c_str(),
            session_id.empty() ? nullptr : session_id.c_str(),
            limit
        );
        
        SearchResult cpp_result;
        cpp_result.total = result.total;
        cpp_result.search_type = result.search_type ? result.search_type : "";
        
        // Free the string allocated by the C library
        if (result.search_type) {
            offline_intelligence_free_string(const_cast<char*>(result.search_type));
        }
        
        return cpp_result;
    }
    
    std::string generate_title(const std::vector<Message>& messages) {
        // Convert C++ messages to C messages
        std::vector<::Message> c_messages;
        c_messages.reserve(messages.size());
        
        for (const auto& msg : messages) {
            ::Message c_msg;
            c_msg.role = msg.role.c_str();
            c_msg.content = msg.content.c_str();
            c_messages.push_back(c_msg);
        }
        
        char* title = offline_intelligence_generate_title(
            handle,
            c_messages.data(),
            static_cast<int32_t>(c_messages.size())
        );
        
        std::string result;
        if (title) {
            result = title;
            offline_intelligence_free_string(title);
        }
        
        return result;
    }
};

} // namespace offline_intelligence

#endif // OFFLINE_INTELLIGENCE_CPP_H