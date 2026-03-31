package com.offlineintelligence;

import java.util.function.Consumer;

public class OfflineIntelligence {

    private final Server server;
    private final Config config;

    public OfflineIntelligence(Config config) {
        this.config = config;
        this.server = new Server(config);
    }

    public OfflineIntelligence() {
        this(new Config());
    }

    public static OfflineIntelligence fromEnv() {
        return new OfflineIntelligence(Config.fromEnv());
    }

    public String healthCheck() throws OfflineIntelligenceException {
        return server.healthCheck();
    }

    public String getStatus() throws OfflineIntelligenceException {
        return server.getStatus();
    }

    public String loadModel(String modelPath) throws OfflineIntelligenceException {
        return server.loadModel(modelPath);
    }

    public String stopModel() throws OfflineIntelligenceException {
        return server.stopModel();
    }

    public String generate(String prompt) throws OfflineIntelligenceException {
        return server.generate(prompt);
    }

    public void generateStream(String prompt, Consumer<String> onChunk) throws OfflineIntelligenceException {
        server.generateStream(prompt, onChunk);
    }

    public String getConversations() throws OfflineIntelligenceException {
        return server.getConversations();
    }

    public String getConversation(String id) throws OfflineIntelligenceException {
        return server.getConversation(id);
    }

    public String deleteConversation(String id) throws OfflineIntelligenceException {
        return server.deleteConversation(id);
    }

    public String getConversationTitle(String id) throws OfflineIntelligenceException {
        return server.getConversationTitle(id);
    }

    public String generateTitle(String sessionId, String firstMessage) throws OfflineIntelligenceException {
        return server.generateTitle(sessionId, firstMessage);
    }

    public String getMemoryStats(String sessionId) throws OfflineIntelligenceException {
        return server.getMemoryStats(sessionId);
    }

    public String optimizeMemory() throws OfflineIntelligenceException {
        return server.optimizeMemory();
    }

    public String cleanupMemory() throws OfflineIntelligenceException {
        return server.cleanupMemory();
    }

    public Config getConfig() { return config; }

    public static String version() { return Server.version(); }

    @Override
    public String toString() {
        return "OfflineIntelligence{baseUrl=http://" +
               config.getApiHost() + ":" + config.getApiPort() + "}";
    }
}
