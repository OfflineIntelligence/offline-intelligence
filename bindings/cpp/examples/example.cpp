#include <offline_intelligence/offline_intelligence.hpp>
#include <iostream>

int main() {
    std::cout << "=== Offline Intelligence C++ Example ===" << std::endl;
    
    try {
        // Create configuration from environment
        auto config = offline_intelligence::Config::from_env();
        
        std::cout << "Configuration loaded:" << std::endl;
        std::cout << "- Model Path: " << config.model_path << std::endl;
        std::cout << "- API Host: " << config.api_host << std::endl;
        std::cout << "- API Port: " << config.api_port << std::endl;
        std::cout << "- Threads: " << config.threads << std::endl;
        std::cout << "- Context Size: " << config.ctx_size << std::endl;
        
        // Start the server
        std::cout << "\nStarting Offline Intelligence server..." << std::endl;
        bool success = offline_intelligence::Server::run_server(config);
        
        if (success) {
            std::cout << "✅ Server started successfully!" << std::endl;
            std::cout << "Server running on: " << config.api_host << ":" << config.api_port << std::endl;
            std::cout << "Version: " << offline_intelligence::Server::version() << std::endl;
        } else {
            std::cout << "❌ Failed to start server" << std::endl;
        }
        
    } catch (const offline_intelligence::OfflineIntelligenceException& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    } catch (const std::exception& e) {
        std::cerr << "Unexpected error: " << e.what() << std::endl;
        return 1;
    }
    
    return 0;
}