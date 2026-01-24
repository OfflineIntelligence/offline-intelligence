const axios = require('axios');

class Config {
    constructor() {
        this.modelPath = 'default.gguf';
        this.llamaBin = 'llama-server';
        this.llamaHost = '127.0.0.1';
        this.llamaPort = 8081;
        this.ctxSize = 8192;
        this.batchSize = 256;
        this.threads = 6;
        this.gpuLayers = 20;
        this.healthTimeoutSeconds = 60;
        this.hotSwapGraceSeconds = 25;
        this.maxConcurrentStreams = 4;
        this.prometheusPort = 9000;
        this.apiHost = '127.0.0.1';
        this.apiPort = 8000;
        this.requestsPerSecond = 24;
        this.generateTimeoutSeconds = 300;
        this.streamTimeoutSeconds = 600;
        this.healthCheckTimeoutSeconds = 90;
        this.queueSize = 100;
        this.queueTimeoutSeconds = 30;
    }

    static fromEnv() {
        const config = new Config();
        // In a real implementation, this would read from process.env
        return config;
    }
}

class OfflineIntelligence {
    constructor(config) {
        this.config = config;
        this.baseUrl = `http://${config.apiHost}:${config.apiPort}`;
    }

    async healthCheck() {
        try {
            const response = await axios.get(`${this.baseUrl}/healthz`);
            return response.data;
        } catch (error) {
            throw new Error(`Health check failed: ${error.message}`);
        }
    }

    async generateStream(prompt, options = {}) {
        try {
            const response = await axios.post(`${this.baseUrl}/generate/stream`, {
                prompt,
                ...options
            }, {
                responseType: 'stream'
            });
            return response.data;
        } catch (error) {
            throw new Error(`Generation failed: ${error.message}`);
        }
    }

    async getStatus() {
        try {
            const response = await axios.get(`${this.baseUrl}/admin/status`);
            return response.data;
        } catch (error) {
            throw new Error(`Status check failed: ${error.message}`);
        }
    }

    async loadModel(modelPath) {
        try {
            const response = await axios.post(`${this.baseUrl}/admin/load`, {
                model_path: modelPath
            });
            return response.data;
        } catch (error) {
            throw new Error(`Model loading failed: ${error.message}`);
        }
    }

    static version() {
        return "0.1.0";
    }
}

module.exports = {
    Config,
    OfflineIntelligence,
    version: OfflineIntelligence.version
};