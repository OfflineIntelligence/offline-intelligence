package com.offlineintelligence;

/**
 * Configuration for the Offline Intelligence HTTP client.
 * Version: 0.1.3
 */
public class Config {
    private String modelPath;
    private String llamaBin;
    private String llamaHost;
    private int llamaPort;
    private String backendUrl;
    private String openrouterApiKey;
    private int ctxSize;
    private int batchSize;
    private int threads;
    private int gpuLayers;
    private long healthTimeoutSeconds;
    private long hotSwapGraceSeconds;
    private int maxConcurrentStreams;
    private int prometheusPort;
    private String apiHost;
    private int apiPort;
    private int requestsPerSecond;
    private long generateTimeoutSeconds;
    private long streamTimeoutSeconds;
    private long healthCheckTimeoutSeconds;
    private int queueSize;
    private long queueTimeoutSeconds;

    public Config() {
        this.modelPath = "default.gguf";
        this.llamaBin = "llama-server";
        this.llamaHost = "127.0.0.1";
        this.llamaPort = 8081;
        this.backendUrl = "http://127.0.0.1:8081";
        this.openrouterApiKey = "";
        this.ctxSize = 8192;
        this.batchSize = 256;
        this.threads = 6;
        this.gpuLayers = 20;
        this.healthTimeoutSeconds = 60;
        this.hotSwapGraceSeconds = 25;
        this.maxConcurrentStreams = 4;
        this.prometheusPort = 9000;
        this.apiHost = "127.0.0.1";
        this.apiPort = 9999;
        this.requestsPerSecond = 24;
        this.generateTimeoutSeconds = 300;
        this.streamTimeoutSeconds = 600;
        this.healthCheckTimeoutSeconds = 90;
        this.queueSize = 100;
        this.queueTimeoutSeconds = 30;
    }

    /** Create a Config populated from environment variables. */
    public static Config fromEnv() {
        Config cfg = new Config();
        String v;
        if ((v = System.getenv("MODEL_PATH")) != null) cfg.modelPath = v;
        if ((v = System.getenv("LLAMA_BIN")) != null) cfg.llamaBin = v;
        if ((v = System.getenv("LLAMA_HOST")) != null) cfg.llamaHost = v;
        if ((v = System.getenv("LLAMA_PORT")) != null) cfg.llamaPort = Integer.parseInt(v);
        if ((v = System.getenv("BACKEND_URL")) != null) cfg.backendUrl = v;
        if ((v = System.getenv("OPENROUTER_API_KEY")) != null) cfg.openrouterApiKey = v;
        if ((v = System.getenv("API_HOST")) != null) cfg.apiHost = v;
        if ((v = System.getenv("API_PORT")) != null) cfg.apiPort = Integer.parseInt(v);
        if ((v = System.getenv("CTX_SIZE")) != null) cfg.ctxSize = Integer.parseInt(v);
        if ((v = System.getenv("GPU_LAYERS")) != null) cfg.gpuLayers = Integer.parseInt(v);
        return cfg;
    }

    // Getters and setters
    public String getModelPath() { return modelPath; }
    public void setModelPath(String modelPath) { this.modelPath = modelPath; }

    public String getLlamaBin() { return llamaBin; }
    public void setLlamaBin(String llamaBin) { this.llamaBin = llamaBin; }

    public String getLlamaHost() { return llamaHost; }
    public void setLlamaHost(String llamaHost) { this.llamaHost = llamaHost; }

    public int getLlamaPort() { return llamaPort; }
    public void setLlamaPort(int llamaPort) { this.llamaPort = llamaPort; }

    public String getBackendUrl() { return backendUrl; }
    public void setBackendUrl(String backendUrl) { this.backendUrl = backendUrl; }

    public String getOpenrouterApiKey() { return openrouterApiKey; }
    public void setOpenrouterApiKey(String openrouterApiKey) { this.openrouterApiKey = openrouterApiKey; }

    public int getCtxSize() { return ctxSize; }
    public void setCtxSize(int ctxSize) { this.ctxSize = ctxSize; }

    public int getBatchSize() { return batchSize; }
    public void setBatchSize(int batchSize) { this.batchSize = batchSize; }

    public int getThreads() { return threads; }
    public void setThreads(int threads) { this.threads = threads; }

    public int getGpuLayers() { return gpuLayers; }
    public void setGpuLayers(int gpuLayers) { this.gpuLayers = gpuLayers; }

    public long getHealthTimeoutSeconds() { return healthTimeoutSeconds; }
    public void setHealthTimeoutSeconds(long healthTimeoutSeconds) { this.healthTimeoutSeconds = healthTimeoutSeconds; }

    public long getHotSwapGraceSeconds() { return hotSwapGraceSeconds; }
    public void setHotSwapGraceSeconds(long hotSwapGraceSeconds) { this.hotSwapGraceSeconds = hotSwapGraceSeconds; }

    public int getMaxConcurrentStreams() { return maxConcurrentStreams; }
    public void setMaxConcurrentStreams(int maxConcurrentStreams) { this.maxConcurrentStreams = maxConcurrentStreams; }

    public int getPrometheusPort() { return prometheusPort; }
    public void setPrometheusPort(int prometheusPort) { this.prometheusPort = prometheusPort; }

    public String getApiHost() { return apiHost; }
    public void setApiHost(String apiHost) { this.apiHost = apiHost; }

    public int getApiPort() { return apiPort; }
    public void setApiPort(int apiPort) { this.apiPort = apiPort; }

    public int getRequestsPerSecond() { return requestsPerSecond; }
    public void setRequestsPerSecond(int requestsPerSecond) { this.requestsPerSecond = requestsPerSecond; }

    public long getGenerateTimeoutSeconds() { return generateTimeoutSeconds; }
    public void setGenerateTimeoutSeconds(long generateTimeoutSeconds) { this.generateTimeoutSeconds = generateTimeoutSeconds; }

    public long getStreamTimeoutSeconds() { return streamTimeoutSeconds; }
    public void setStreamTimeoutSeconds(long streamTimeoutSeconds) { this.streamTimeoutSeconds = streamTimeoutSeconds; }

    public long getHealthCheckTimeoutSeconds() { return healthCheckTimeoutSeconds; }
    public void setHealthCheckTimeoutSeconds(long healthCheckTimeoutSeconds) { this.healthCheckTimeoutSeconds = healthCheckTimeoutSeconds; }

    public int getQueueSize() { return queueSize; }
    public void setQueueSize(int queueSize) { this.queueSize = queueSize; }

    public long getQueueTimeoutSeconds() { return queueTimeoutSeconds; }
    public void setQueueTimeoutSeconds(long queueTimeoutSeconds) { this.queueTimeoutSeconds = queueTimeoutSeconds; }

    @Override
    public String toString() {
        return "Config{apiHost='" + apiHost + "', apiPort=" + apiPort +
               ", modelPath='" + modelPath + "'}";
    }
}