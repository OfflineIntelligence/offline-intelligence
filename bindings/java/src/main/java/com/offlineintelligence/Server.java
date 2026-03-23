package com.offlineintelligence;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;
import java.util.function.Consumer;

/**
 * HTTP client for the Offline Intelligence server.
 * Version: 0.1.3
 *
 * <p>All methods communicate with the running Offline Intelligence server
 * at http://{apiHost}:{apiPort} (default port 9999).</p>
 */
public class Server {

    private static final String VERSION = "0.1.3";
    private static final ObjectMapper MAPPER = new ObjectMapper();

    private final Config config;
    private final HttpClient http;
    private final String baseUrl;

    public Server(Config config) {
        this.config = config;
        this.baseUrl = "http://" + config.getApiHost() + ":" + config.getApiPort();
        this.http = HttpClient.newBuilder()
            .version(HttpClient.Version.HTTP_1_1)
            .connectTimeout(Duration.ofSeconds(config.getHealthTimeoutSeconds()))
            .build();
    }

    // ── Internal helpers ─────────────────────────────────────────────────

    private String get(String path) throws OfflineIntelligenceException {
        try {
            HttpRequest req = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + path))
                .timeout(Duration.ofSeconds(config.getHealthTimeoutSeconds()))
                .GET()
                .build();
            HttpResponse<String> resp = http.send(req, HttpResponse.BodyHandlers.ofString());
            if (resp.statusCode() >= 400) {
                throw new OfflineIntelligenceException("HTTP " + resp.statusCode() + ": " + resp.body());
            }
            return resp.body();
        } catch (OfflineIntelligenceException e) {
            throw e;
        } catch (Exception e) {
            throw new OfflineIntelligenceException("Request failed: " + e.getMessage(), e);
        }
    }

    private String post(String path, String jsonBody) throws OfflineIntelligenceException {
        try {
            HttpRequest req = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + path))
                .timeout(Duration.ofSeconds(config.getGenerateTimeoutSeconds()))
                .header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(jsonBody != null ? jsonBody : "{}"))
                .build();
            HttpResponse<String> resp = http.send(req, HttpResponse.BodyHandlers.ofString());
            if (resp.statusCode() >= 400) {
                throw new OfflineIntelligenceException("HTTP " + resp.statusCode() + ": " + resp.body());
            }
            return resp.body();
        } catch (OfflineIntelligenceException e) {
            throw e;
        } catch (Exception e) {
            throw new OfflineIntelligenceException("Request failed: " + e.getMessage(), e);
        }
    }

    private String delete(String path) throws OfflineIntelligenceException {
        try {
            HttpRequest req = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + path))
                .timeout(Duration.ofSeconds(config.getHealthTimeoutSeconds()))
                .DELETE()
                .build();
            HttpResponse<String> resp = http.send(req, HttpResponse.BodyHandlers.ofString());
            if (resp.statusCode() >= 400) {
                throw new OfflineIntelligenceException("HTTP " + resp.statusCode() + ": " + resp.body());
            }
            return resp.body();
        } catch (OfflineIntelligenceException e) {
            throw e;
        } catch (Exception e) {
            throw new OfflineIntelligenceException("Request failed: " + e.getMessage(), e);
        }
    }

    // ── Health & Status ──────────────────────────────────────────────────

    /** GET /healthz */
    public String healthCheck() throws OfflineIntelligenceException {
        return get("/healthz");
    }

    /** GET /admin/status */
    public String getStatus() throws OfflineIntelligenceException {
        return get("/admin/status");
    }

    // ── Model Management ─────────────────────────────────────────────────

    /** POST /admin/load */
    public String loadModel(String modelPath) throws OfflineIntelligenceException {
        ObjectNode body = MAPPER.createObjectNode();
        body.put("model_path", modelPath);
        return post("/admin/load", body.toString());
    }

    /** POST /admin/stop */
    public String stopModel() throws OfflineIntelligenceException {
        return post("/admin/stop", null);
    }

    // ── Generation ───────────────────────────────────────────────────────

    /** POST /generate */
    public String generate(String prompt) throws OfflineIntelligenceException {
        ObjectNode body = MAPPER.createObjectNode();
        body.put("prompt", prompt);
        return post("/generate", body.toString());
    }

    /**
     * POST /generate/stream — streams SSE response, invoking onChunk for each token.
     *
     * @param prompt   The prompt text.
     * @param onChunk  Callback invoked for each streamed text chunk.
     */
    public void generateStream(String prompt, Consumer<String> onChunk) throws OfflineIntelligenceException {
        ObjectNode body = MAPPER.createObjectNode();
        body.put("prompt", prompt);
        try {
            HttpRequest req = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/generate/stream"))
                .timeout(Duration.ofSeconds(config.getStreamTimeoutSeconds()))
                .header("Content-Type", "application/json")
                .header("Accept", "text/event-stream")
                .POST(HttpRequest.BodyPublishers.ofString(body.toString()))
                .build();
            HttpResponse<java.io.InputStream> resp =
                http.send(req, HttpResponse.BodyHandlers.ofInputStream());
            if (resp.statusCode() >= 400) {
                throw new OfflineIntelligenceException("HTTP " + resp.statusCode());
            }
            try (BufferedReader reader = new BufferedReader(new InputStreamReader(resp.body()))) {
                String line;
                while ((line = reader.readLine()) != null) {
                    if (line.startsWith("data: ")) {
                        String data = line.substring(6);
                        if ("[DONE]".equals(data.trim())) break;
                        try {
                            var node = MAPPER.readTree(data);
                            if (node.has("choices")) {
                                String content = node.path("choices").get(0)
                                    .path("delta").path("content").asText("");
                                if (!content.isEmpty()) onChunk.accept(content);
                            } else if (node.has("text")) {
                                onChunk.accept(node.get("text").asText());
                            }
                        } catch (Exception ignored) {
                            onChunk.accept(data);
                        }
                    }
                }
            }
        } catch (OfflineIntelligenceException e) {
            throw e;
        } catch (Exception e) {
            throw new OfflineIntelligenceException("Stream failed: " + e.getMessage(), e);
        }
    }

    // ── Conversations ────────────────────────────────────────────────────

    /** GET /conversations */
    public String getConversations() throws OfflineIntelligenceException {
        return get("/conversations");
    }

    /** GET /conversations/{id} */
    public String getConversation(String id) throws OfflineIntelligenceException {
        return get("/conversations/" + id);
    }

    /** DELETE /conversations/{id} */
    public String deleteConversation(String id) throws OfflineIntelligenceException {
        return delete("/conversations/" + id);
    }

    /** GET /conversations/{id}/title */
    public String getConversationTitle(String id) throws OfflineIntelligenceException {
        return get("/conversations/" + id + "/title");
    }

    /** POST /generate/title */
    public String generateTitle(String sessionId, String firstMessage) throws OfflineIntelligenceException {
        ObjectNode body = MAPPER.createObjectNode();
        body.put("session_id", sessionId);
        body.put("first_message", firstMessage);
        return post("/generate/title", body.toString());
    }

    // ── Memory ───────────────────────────────────────────────────────────

    /** GET /memory/stats/{session_id} */
    public String getMemoryStats(String sessionId) throws OfflineIntelligenceException {
        return get("/memory/stats/" + sessionId);
    }

    /** POST /memory/optimize */
    public String optimizeMemory() throws OfflineIntelligenceException {
        return post("/memory/optimize", null);
    }

    /** POST /memory/cleanup */
    public String cleanupMemory() throws OfflineIntelligenceException {
        return post("/memory/cleanup", null);
    }

    // ── Legacy / Static ──────────────────────────────────────────────────

    /**
     * @deprecated Use {@link OfflineIntelligence} instead.
     *             This method now starts the client and performs a health check.
     */
    @Deprecated
    public static boolean runServer(Config config) throws OfflineIntelligenceException {
        Server server = new Server(config);
        System.out.println("Connecting to Offline Intelligence server...");
        System.out.println("API: " + server.baseUrl);
        server.healthCheck();
        System.out.println("Server is healthy.");
        return true;
    }

    /** @return library version string */
    public static String version() {
        return VERSION;
    }
}