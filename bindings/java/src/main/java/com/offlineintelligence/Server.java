package com.offlineintelligence;

/**
 * Main server class for Offline Intelligence
 */
public class Server {
    
    /**
     * Start the Offline Intelligence server
     * @param config Server configuration
     * @return true if server started successfully
     * @throws OfflineIntelligenceException if server fails to start
     */
    public static boolean runServer(Config config) throws OfflineIntelligenceException {
        try {
            System.out.println("Starting Offline Intelligence server...");
            System.out.println("API Server: " + config.getApiHost() + ":" + config.getApiPort());
            System.out.println("LLM Backend: " + config.getLlamaHost() + ":" + config.getLlamaPort());
            System.out.println("Model: " + config.getModelPath());
            System.out.println("Context Size: " + config.getCtxSize());
            System.out.println("Threads: " + config.getThreads());
            System.out.println("GPU Layers: " + config.getGpuLayers());
            
            // In a real implementation, this would:
            // 1. Load the native library
            // 2. Initialize the Rust components
            // 3. Start the HTTP server
            
            return true;
        } catch (Exception e) {
            throw new OfflineIntelligenceException("Failed to start server: " + e.getMessage(), e);
        }
    }
    
    /**
     * Get library version
     * @return Version string
     */
    public static String version() {
        return "0.1.2";
    }
}