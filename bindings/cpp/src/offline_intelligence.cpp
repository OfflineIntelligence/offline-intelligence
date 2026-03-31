#include "offline_intelligence.h"
#include <iostream>
#include <stdexcept>

namespace offline_intelligence {

Config Config::from_env() {
    Config cfg;
    cfg.model_path = "default.gguf";
    cfg.llama_bin = "llama-server";
    cfg.llama_host = "127.0.0.1";
    cfg.llama_port = 8081;
    cfg.ctx_size = 8192;
    cfg.batch_size = 256;
    cfg.threads = 6;
    cfg.gpu_layers = 20;
    cfg.health_timeout_seconds = 60;
    cfg.hot_swap_grace_seconds = 25;
    cfg.max_concurrent_streams = 4;
    cfg.prometheus_port = 9000;
    cfg.api_host = "127.0.0.1";
    cfg.api_port = 8000;
    cfg.requests_per_second = 24;
    cfg.generate_timeout_seconds = 300;
    cfg.stream_timeout_seconds = 600;
    cfg.health_check_timeout_seconds = 90;
    cfg.queue_size = 100;
    cfg.queue_timeout_seconds = 30;
    return cfg;
}

bool Server::run_server(const Config& config) {
    try {
        std::cout << "Starting Offline Intelligence server..." << std::endl;
        std::cout << "API Server: " << config.api_host << ":" << config.api_port << std::endl;
        std::cout << "LLM Backend: " << config.llama_host << ":" << config.llama_port << std::endl;
        std::cout << "Model: " << config.model_path << std::endl;
        std::cout << "Context Size: " << config.ctx_size << std::endl;
        std::cout << "Threads: " << config.threads << std::endl;
        std::cout << "GPU Layers: " << config.gpu_layers << std::endl;
        
        return true;
    } catch (const std::exception& e) {
        throw OfflineIntelligenceException(std::string("Failed to start server: ") + e.what());
    }
}

std::string Server::version() {
    return "0.1.0";
}

} 
