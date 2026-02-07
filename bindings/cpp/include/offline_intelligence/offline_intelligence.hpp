#ifndef OFFLINE_INTELLIGENCE_H
#define OFFLINE_INTELLIGENCE_H

#include <string>
#include <cstdint>
#include <memory>
#include <iostream>
#include <stdexcept>

namespace offline_intelligence {

/**
 * @brief Exception class for Offline Intelligence errors
 */
class OfflineIntelligenceException : public std::exception {
private:
    std::string message_;
    
public:
    explicit OfflineIntelligenceException(const std::string& message) 
        : message_(message) {}
    
    const char* what() const noexcept override {
        return message_.c_str();
    }
};

/**
 * @brief Configuration structure for the Offline Intelligence engine
 */
struct Config {
    std::string model_path = "default.gguf";
    std::string llama_bin = "llama-server";
    std::string llama_host = "127.0.0.1";
    uint16_t llama_port = 8081;
    uint32_t ctx_size = 8192;
    uint32_t batch_size = 256;
    uint32_t threads = 6;
    uint32_t gpu_layers = 20;
    uint64_t health_timeout_seconds = 60;
    uint64_t hot_swap_grace_seconds = 25;
    uint32_t max_concurrent_streams = 4;
    uint16_t prometheus_port = 9000;
    std::string api_host = "127.0.0.1";
    uint16_t api_port = 8000;
    uint32_t requests_per_second = 24;
    uint64_t generate_timeout_seconds = 300;
    uint64_t stream_timeout_seconds = 600;
    uint64_t health_check_timeout_seconds = 90;
    size_t queue_size = 100;
    uint64_t queue_timeout_seconds = 30;

    /**
     * @brief Create configuration from environment variables
     * @return Config populated with environment values
     */
    static Config from_env() {
        Config cfg;
        // In a real implementation, this would read from environment variables
        // For now, return default configuration
        return cfg;
    }
};

/**
 * @brief Main server class
 */
class Server {
public:
    /**
     * @brief Start the Offline Intelligence server
     * @param config Server configuration
     * @return true if server started successfully, false otherwise
     */
    static bool run_server(const Config& config) {
        try {
            std::cout << "Starting Offline Intelligence server..." << std::endl;
            std::cout << "API Server: " << config.api_host << ":" << config.api_port << std::endl;
            std::cout << "LLM Backend: " << config.llama_host << ":" << config.llama_port << std::endl;
            std::cout << "Model: " << config.model_path << std::endl;
            std::cout << "Context Size: " << config.ctx_size << std::endl;
            std::cout << "Threads: " << config.threads << std::endl;
            std::cout << "GPU Layers: " << config.gpu_layers << std::endl;
            
            // In a real implementation, this would:
            // 1. Initialize the Rust library via FFI
            // 2. Start the HTTP server
            // 3. Begin processing requests
            
            return true;
        } catch (const std::exception& e) {
            throw OfflineIntelligenceException(std::string("Failed to start server: ") + e.what());
        }
    }
    
    /**
     * @brief Get library version
     * @return Version string
     */
    static std::string version() {
        return "0.1.2";
    }
};

} // namespace offline_intelligence

#endif // OFFLINE_INTELLIGENCE_H