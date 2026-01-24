package examples;

import com.offlineintelligence.Config;
import com.offlineintelligence.Server;
import com.offlineintelligence.OfflineIntelligenceException;

/**
 * Example usage of Offline Intelligence Java Bindings
 */
public class OfflineIntelligenceExample {
    public static void main(String[] args) {
        System.out.println("=== Offline Intelligence Java Example ===");
        
        try {
            // Create configuration from environment
            Config config = Config.fromEnv();
            
            System.out.println("Configuration loaded:");
            System.out.println("- Model Path: " + config.getModelPath());
            System.out.println("- API Host: " + config.getApiHost());
            System.out.println("- API Port: " + config.getApiPort());
            System.out.println("- Threads: " + config.getThreads());
            System.out.println("- Context Size: " + config.getCtxSize());
            
            // Start the server
            System.out.println("\nStarting Offline Intelligence server...");
            boolean success = Server.runServer(config);
            
            if (success) {
                System.out.println("✅ Server started successfully!");
                System.out.println("Server running on: " + config.getApiHost() + ":" + config.getApiPort());
                System.out.println("Version: " + Server.version());
            } else {
                System.out.println("❌ Failed to start server");
            }
            
        } catch (OfflineIntelligenceException e) {
            System.err.println("Error: " + e.getMessage());
            e.printStackTrace();
        } catch (Exception e) {
            System.err.println("Unexpected error: " + e.getMessage());
            e.printStackTrace();
        }
    }
}