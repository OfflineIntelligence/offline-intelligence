package com.offlineintelligence;

/**
 * Configuration class for Offline Intelligence engine
 */
public class Config {
    private String modelPath;
    private String llamaBin;
    private String llamaHost;
    private int llamaPort;
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

    // Constructor
    public Config() {
        this.modelPath = "default.gguf";
        this.llamaBin = "llama-server";
        this.llamaHost = "127.0.0.1";
        this.llamaPort = 8081;
        this.ctxSize = 8192;
        this.batchSize = 256;
        this.threads = 6;
        this.gpuLayers = 20;
        this.healthTimeoutSeconds = 60;
        this.hotSwapGraceSeconds = 25;
        this.maxConcurrentStreams = 4;
        this.prometheusPort = 9000;
        this.apiHost = "127.0.0.1";
        this.apiPort = 8000;
        this.requestsPerSecond = 24;
        this.generateTimeoutSeconds = 300;
        this.streamTimeoutSeconds = 600;
        this.healthCheckTimeoutSeconds = 90;
        this.queueSize = 100;
        this.queueTimeoutSeconds = 30;
    }

    // Static factory method
    public static Config fromEnv() {
        return new Config();
        // In real implementation, would read from System.getenv()
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
}