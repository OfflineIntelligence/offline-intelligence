#ifndef OFFLINE_INTELLIGENCE_H
#define OFFLINE_INTELLIGENCE_H

#include <string>
#include <cstdint>
#include <memory>

namespace offline_intelligence {

/**
 * @brief Configuration structure for the Offline Intelligence engine
 */
struct Config {
    std::string model_path;
    std::string llama_bin;
    std::string llama_host;
    uint16_t llama_port;
    uint32_t ctx_size;
    uint32_t batch_size;
    uint32_t threads;
    uint32_t gpu_layers;
    uint64_t health_timeout_seconds;
    uint64_t hot_swap_grace_seconds;
    uint32_t max_concurrent_streams;
    uint16_t prometheus_port;
    std::string api_host;
    uint16_t api_port;
    uint32_t requests_per_second;
    uint64_t generate_timeout_seconds;
    uint64_t stream_timeout_seconds;
    uint64_t health_check_timeout_seconds;
    size_t queue_size;
    uint64_t queue_timeout_seconds;

    /**
     * @brief Create configuration from environment variables
     * @return Config populated with environment values
     */
    static Config from_env();
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
    static bool run_server(const Config& config);
    
    /**
     * @brief Get library version
     * @return Version string
     */
    static std::string version();
};

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

} // namespace offline_intelligence

#endif // OFFLINE_INTELLIGENCE_H